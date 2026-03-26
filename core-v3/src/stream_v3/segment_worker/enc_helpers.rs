// ## 📦 `src/stream_v3/segment_worker/enc_helpers.rs`
use std::time::Instant;
use bytes::Bytes;
use crossbeam::channel::{Receiver, Sender};
use tracing::{debug, warn};

use core_api::{
    crypto::{DigestAlg, DigestFrame, SegmentDigestBuilder}, 
    stream_v2::{frame_worker::{EncryptedFrame, FrameInput}, framing::FrameType, segment_worker::{EncryptedSegment, SegmentWorkerError}, segmenting::{SegmentHeader, types::SegmentFlags}},
    telemetry::{Stage, StageTimes, TelemetryCounters, TelemetryEvent}, types::StreamError, utils::tracing_logger
};

use  crate::stream_v3::{pipeline::{Monitor, PipelineMonitor}, segment_worker::SegmentInput};

/// Processes a single plaintext segment into encrypted wire format
///
/// # Process Flow
/// 1. Validates input and handles empty final segments
/// 2. Splits plaintext into frame-sized chunks
/// 3. Dispatches frames to worker pool for parallel encryption
/// 4. Collects and sorts encrypted frames
/// 5. Computes segment digest over all frame ciphertext
/// 6. Creates digest frame and terminator frame
/// 7. Serializes all frames into wire format with segment header
///
/// # Arguments
/// * `input` - Input segment containing plaintext and metadata
/// * `frame_size` - Maximum size of plaintext per frame
/// * `digest_alg` - Digest algorithm for segment integrity
/// * `frame_tx` - Channel to dispatch frames to worker pool
/// * `out_rx` - Channel to collect encrypted frames from workers
/// * `cancelled` - Cancellation flag for early exit

pub fn process_encrypt_segment_3(
    input: &SegmentInput,
    frame_size  : usize,
    digest_alg  : DigestAlg,
    frame_tx    : &Sender<FrameInput>,
    out_rx      : &Receiver<EncryptedFrame>,   // 👈 clean frames only
    monitor     : Monitor,
) -> Option<EncryptedSegment> {
    tracing_logger(Some(tracing::Level::DEBUG));

    let mut counters = TelemetryCounters::default();
    let mut stage_times = StageTimes::default();

    debug!("[ENCRYPT SEGMENT] processing segment {}", input.index);

    // ---- Stage 1: Validation ----
    let start = Instant::now();
    if input.bytes.is_empty() && input.flags.contains(SegmentFlags::FINAL_SEGMENT) {
        debug!("[ENCRYPT SEGMENT] empty FINAL_SEGMENT at index {}", input.index);
        let header = SegmentHeader::new(
            &Bytes::new(),
            input.index,
            0,
            0,
            digest_alg as u16,
            input.flags,
        );
        return Some(EncryptedSegment {
            header,
            wire: Bytes::new(),
            counters,
            stage_times,
        });
    }

    counters.add_header(SegmentHeader::LEN);

    let bytes_len = input.bytes.len();
    let frame_count = (bytes_len + frame_size - 1) / frame_size;
    if frame_count == 0 {
        monitor.report_error(StreamError::SegmentWorker(
            SegmentWorkerError::InvalidSegment("Empty segment without FINAL_SEGMENT flag".into())
        ));
        return None;
    }

    stage_times.add(Stage::Validate, start.elapsed());

    // ---- Stage 2: Dispatch frames ----
    let start = Instant::now();
    debug!("[ENCRYPT SEGMENT] dispatching {} frames", frame_count);

    let all_bytes = &input.bytes;
    // let cancel_s = monitor.cancel_rx().clone();
    // for (frame_index, chunk) in all_bytes.chunks(frame_size).enumerate() {
    //     // Bound frame size
    //     let start = frame_index * frame_size;
    //     let end = start + chunk.len();

    //     let frame_input = FrameInput {
    //         segment_index: input.index,
    //         frame_index: frame_index as u32,
    //         frame_type: FrameType::Data,
    //         payload: all_bytes.slice(start..end),
    //     };

    //     // Now cancellable send with exact frame slice
    //     crossbeam::select! {
    //         recv(cancel_s) -> _ => {
    //             warn!("[ENCRYPT SEGMENT] cancelled during frame dispatch, exiting");
    //             return None;
    //         }
    //         send(frame_tx, frame_input) -> result => {
    //             if let Err(e) = result {
    //                 monitor.report_error(StreamError::SegmentWorker(SegmentWorkerError::StateError(e.to_string())));
    //                 return None;
    //             }
    //         }
    //         default(std::time::Duration::from_millis(10)) => {
    //             if monitor.is_cancelled() {
    //                 warn!("[ENCRYPT SEGMENT] cancelled (timeout path), exiting");
    //                 return None;
    //             }
    //         }
    //     }
    // }
    for (frame_index, chunk) in all_bytes.chunks(frame_size).enumerate() {
        // Compute the offset of this chunk relative to the original buffer
        let start = frame_index * frame_size;
        let end = start + chunk.len();

        let frame_input = FrameInput {
            segment_index: input.index,
            frame_index: frame_index as u32,
            frame_type: FrameType::Data,
            payload: all_bytes.slice(start..end),
        };

        if let Err(e) = frame_tx.send(frame_input) {
            monitor.report_error(StreamError::SegmentWorker(SegmentWorkerError::StateError(e.to_string())));
            return None;
        }
    }

    stage_times.add(Stage::Read, start.elapsed());

    // ---- Stage 3: Collect frames ----
    let mut data_frames = vec![EncryptedFrame::default(); frame_count];
    let mut data_wire_len = 0;
    let mut received = 0;

    let cancel_r = monitor.cancel_rx().clone();
    while received < frame_count {
        crossbeam::select! {
            recv(out_rx) -> result => {
                match result {
                    Ok(frame) => {
                        let idx = frame.frame_index;
                        received += 1;
                        stage_times.merge(&frame.stage_times);
                        data_frames[idx as usize] = frame;
                    }
                    Err(e) => {
                        monitor.report_error(StreamError::SegmentWorker(SegmentWorkerError::StateError(e.to_string())));
                        return None;
                    }
                }
            }
            recv(cancel_r) -> _ => {
                warn!("[ENCRYPT SEGMENT] cancelled, exiting");
                return None;
            }
            default(std::time::Duration::from_millis(10)) => {
                if monitor.is_cancelled() {
                    warn!("[ENCRYPT SEGMENT] cancelled (timeout path), exiting");
                    return None;
                }
            }
        }
    }

    // ---- Stage 4: Digest ----
    let start = Instant::now();
    let mut digest_builder = SegmentDigestBuilder::new(digest_alg, input.index, frame_count as u32);

    for frame in &data_frames {
        data_wire_len += frame.wire.len();
        counters.bytes_overhead += EncryptedFrame::frame_overhead() as u64;
        counters.bytes_ciphertext += frame.ciphertext().len() as u64;
        digest_builder.update_frame(frame.frame_index, frame.ciphertext());
    }

    counters.frames_data = frame_count as u64;

    let digest = match digest_builder.finalize() {
        Ok(d) => d,
        Err(e) => {
            monitor.report_error(StreamError::SegmentWorker(SegmentWorkerError::DigestError(e)));
            return None;
        }
    };
    let digest_payload = Bytes::from(DigestFrame::new(digest_alg, digest).encode());

    if let Err(e) = frame_tx.send(FrameInput {
        segment_index: input.index,
        frame_index: frame_count as u32,
        frame_type: FrameType::Digest,
        payload: digest_payload,
    }) {
        monitor.report_error(StreamError::SegmentWorker(SegmentWorkerError::StateError(e.to_string())));
        return None;
    }

    let digest_frame = crossbeam::select! {
        recv(out_rx) -> result => {
            match result {
                Ok(frame) => frame,
                Err(e) => {
                    monitor.report_error(StreamError::SegmentWorker(SegmentWorkerError::StateError(e.to_string())));
                    return None;
                }
            }
        }
        recv(cancel_r) -> _ => { return None; }
    };

    stage_times.add(Stage::Digest, start.elapsed());
    counters.add_digest(digest_frame.ciphertext().len());

    // ---- Stage 5: Terminator ----
    let start = Instant::now();
    if let Err(e) = frame_tx.send(FrameInput {
        segment_index: input.index,
        frame_index: frame_count as u32 + 1,
        frame_type: FrameType::Terminator,
        payload: Bytes::new(),
    }) {
        monitor.report_error(StreamError::SegmentWorker(SegmentWorkerError::StateError(e.to_string())));
        return None;
    }

    let terminator_frame = crossbeam::select! {
        recv(out_rx) -> result => {
            match result {
                Ok(frame) => frame,
                Err(e) => {
                    monitor.report_error(StreamError::SegmentWorker(SegmentWorkerError::StateError(e.to_string())));
                    return None;
                }
            }
        }
        recv(cancel_r) -> _ => { return None; }
    };

    counters.add_terminator(terminator_frame.ciphertext().len());
    stage_times.add(Stage::Validate, start.elapsed());

    // ---- Stage 6: Serialize ----
    let start = Instant::now();
    let total_len = data_wire_len + digest_frame.wire.len() + terminator_frame.wire.len();
    let mut wire_bytes = Vec::with_capacity(total_len);

    for frame in data_frames {
        wire_bytes.extend_from_slice(&frame.wire);
    }
    wire_bytes.extend_from_slice(&digest_frame.wire);
    wire_bytes.extend_from_slice(&terminator_frame.wire);

    let wire = Bytes::from(wire_bytes);

    let header = SegmentHeader::new(
        &wire,
        input.index,
        bytes_len as u32,
        frame_count as u32,
        digest_alg as u16,
        input.flags,
    );

    stage_times.add(Stage::Write, start.elapsed());

    debug!(
        "[ENCRYPT SEGMENT] completed segment {} ({} bytes -> {} frames)",
        input.index, bytes_len, frame_count
    );

    // Report telemetry snapshot
    monitor.report_telemetry(TelemetryEvent::StageSnapshot {
        stage_times: stage_times.clone(),
        counters: counters.clone(),
    });

    Some(EncryptedSegment {
        header,
        wire,
        counters,
        stage_times,
    })
}
