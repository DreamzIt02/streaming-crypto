// ## 📦 `src/stream_v3/segment_worker/dec_helpers.rs`

use std::time::Instant;

use bytes::Bytes;
use crossbeam::channel::{Receiver, Sender};
use tracing::{debug, warn};

use core_api::{
    crypto::{DigestAlg, DigestFrame, SegmentDigestVerifier}, 
    stream::{frame_worker::DecryptedFrame, framing::{FrameType, FrameError, FrameHeader}, segment_worker::{DecryptedSegment, SegmentWorkerError}, segmenting::{SegmentHeader, types::SegmentFlags}}, 
    telemetry::{Stage, StageTimes, TelemetryCounters, TelemetryEvent}, types::StreamError, utils::tracing_logger
};

use crate::stream_v3::{pipeline::{Monitor, PipelineMonitor}, segment_worker::SegmentInput};

/// Processes a single encrypted segment into plaintext
///
/// # Process Flow
/// 1. Validates segment header and handles empty final segments
/// 2. Parses frame boundaries from wire format (zero-copy slicing)
/// 3. Dispatches frame slices to worker pool for parallel decryption
/// 4. Collects and sorts decrypted data frames
/// 5. Verifies segment digest over all frame ciphertext
/// 6. Validates terminator frame
/// 7. Reassembles plaintext from all data frames
///
/// # Arguments
/// * `input` - Input segment containing encrypted wire data and header
/// * `digest_alg` - Digest algorithm for segment integrity verification
/// * `frame_tx` - Channel to dispatch frame slices to worker pool
/// * `out_rx` - Channel to collect decrypted frames from workers
/// * `cancelled` - Cancellation flag for early exit

pub fn process_decrypt_segment_3(
    input       : &SegmentInput,
    digest_alg  : &DigestAlg,
    frame_tx    : &Sender<Bytes>,
    out_rx      : &Receiver<DecryptedFrame>,   // 👈 clean frames only
    monitor     : Monitor,
) -> Option<DecryptedSegment> {
    tracing_logger(Some(tracing::Level::DEBUG));

    let mut counters: TelemetryCounters = TelemetryCounters::default();
    let mut stage_times = StageTimes::default();

    debug!("[DECRYPT SEGMENT] processing segment {}", input.header.segment_index());

    // ---- Stage 1: Validation ----
    let start = Instant::now();
    if input.bytes.is_empty() && input.header.flags().contains(SegmentFlags::FINAL_SEGMENT) {
        debug!("[DECRYPT SEGMENT] empty FINAL_SEGMENT at index {}", input.header.segment_index());
        return Some(DecryptedSegment {
            header: input.header.clone(),
            bytes: Bytes::new(),
            counters,
            stage_times,
        });
    }

    if let Err(e) = input.header.validate(&input.bytes) {
        monitor.report_error(StreamError::Segment(e));
        return None;
    }

    stage_times.add(Stage::Validate, start.elapsed());
    counters.add_header(SegmentHeader::LEN);

    // ---- Stage 2: Parse frames and dispatch ----
    let start = Instant::now();
    let mut offset = 0;
    let mut frame_count: usize = 0;

    let mut verifier = SegmentDigestVerifier::new(
        digest_alg.clone(),
        input.header.segment_index(),
        input.header.frame_count(),
    );

    // let cancel_s = monitor.cancel_rx().clone();
    // while offset < input.bytes.len() {
    //     // Parse the frame header first
    //     let header = match FrameHeader::from_bytes(&input.bytes[offset..]) {
    //         Ok(h) => h,
    //         Err(e) => {
    //             monitor.report_error(StreamError::SegmentWorker(SegmentWorkerError::FramingError(e)));
    //             return None;
    //         }
    //     };
    //     let frame_len = FrameHeader::LEN + header.ciphertext_len() as usize;
    //     let end = offset + frame_len;

    //     if end > input.bytes.len() {
    //         monitor.report_error(StreamError::SegmentWorker(SegmentWorkerError::FrameWorkerError(
    //             FrameError::Truncated.into(),
    //         )));
    //         return None;
    //     }

    //     // Update verifier for data frames
    //     if header.frame_type() == FrameType::Data {
    //         let ct_start = offset + FrameHeader::LEN;
    //         let ct_end = ct_start + header.ciphertext_len() as usize;
    //         verifier.update_frame(header.frame_index(), &input.bytes[ct_start..ct_end]);
    //     }

    //     if let Err(e) = frame_tx.send(input.bytes.slice(offset..end)) {
    //         monitor.report_error(StreamError::SegmentWorker(SegmentWorkerError::StateError(e.to_string())));
    //         return None;
    //     }

    //     // Now cancellable send with exact frame slice
    //     crossbeam::select! {
    //         recv(cancel_s) -> _ => {
    //             warn!("[DECRYPT SEGMENT] cancelled during frame dispatch, exiting");
    //             return None;
    //         }
    //         send(frame_tx, input.bytes.slice(offset..end)) -> result => {
    //             if let Err(e) = result {
    //                 monitor.report_error(StreamError::SegmentWorker(SegmentWorkerError::StateError(e.to_string())));
    //                 return None;
    //             }
    //         }
    //         default(std::time::Duration::from_millis(10)) => {
    //             if monitor.is_cancelled() {
    //                 warn!("[DECRYPT SEGMENT] cancelled (timeout path), exiting");
    //                 return None;
    //             }
    //         }
    //     }

    //     offset = end;
    //     frame_count += 1;
    // }
    while offset < input.bytes.len() {
        // Parse frame header to determine frame length
        let header = match FrameHeader::from_bytes(&input.bytes[offset..]) {
            Ok(h) => h,
            Err(e) => {
                monitor.report_error(StreamError::SegmentWorker(SegmentWorkerError::FramingError(e)));
                return None;
            }
        };
        let frame_len = FrameHeader::LEN + header.ciphertext_len() as usize;
        let end = offset + frame_len;

        // Validate frame doesn't extend beyond wire boundary
        if end > input.bytes.len() {
            monitor.report_error(StreamError::SegmentWorker(SegmentWorkerError::FrameWorkerError(
                FrameError::Truncated.into(),
            )));
            return None;
        }

        // Update verifier for data frames
        if header.frame_type() == FrameType::Data {
            let ct_start = offset + FrameHeader::LEN;
            let ct_end = ct_start + header.ciphertext_len() as usize;
            verifier.update_frame(header.frame_index(), &input.bytes[ct_start..ct_end]);
        }

        // Dispatch frame slice for decryption (zero-copy using Bytes::slice)
        if let Err(e) = frame_tx.send(input.bytes.slice(offset..end)) {
            monitor.report_error(StreamError::SegmentWorker(SegmentWorkerError::StateError(e.to_string())));
            return None;
        }

        offset = end;
        frame_count += 1;
    }

    let digest_actual = match verifier.finalize() {
        Ok(d) => d,
        Err(e) => {
            monitor.report_error(StreamError::SegmentWorker(SegmentWorkerError::DigestError(e)));
            return None;
        }
    };

    stage_times.add(Stage::Read, start.elapsed());

    if frame_count == 0 {
        monitor.report_error(StreamError::SegmentWorker(SegmentWorkerError::InvalidSegment(
            "Segment contains no frames".into(),
        )));
        return None;
    }

    // ---- Stage 3: Collect decrypted frames ----
    let mut data_frames = vec![DecryptedFrame::default(); frame_count.saturating_sub(2)];
    let mut digest_frame: Option<DecryptedFrame> = None;
    let mut terminator_frame: Option<DecryptedFrame> = None;
    let mut received = 0;

    // Stage 3 start  
    let cancel_r = monitor.cancel_rx().clone();
    while received < frame_count {
        crossbeam::select! {
            recv(out_rx) -> result => {
                match result {
                    Ok(frame) => {
                        received += 1;
                     
                        stage_times.merge(&frame.stage_times);
                        match frame.frame_type {
                            FrameType::Data => {
                                let frame_index = frame.frame_index as usize;
                                data_frames[frame_index] = frame
                            },
                            FrameType::Digest => {
                                if digest_frame.is_some() {
                                    monitor.report_error(StreamError::SegmentWorker(SegmentWorkerError::InvalidSegment(
                                        "Multiple digest frames detected".into(),
                                    )));
                                    return None;
                                }
                                digest_frame = Some(frame);
                            }
                            FrameType::Terminator => {
                                if terminator_frame.is_some() {
                                    monitor.report_error(StreamError::SegmentWorker(SegmentWorkerError::InvalidSegment(
                                        "Multiple terminator frames detected".into(),
                                    )));
                                    return None;
                                }
                                terminator_frame = Some(frame);
                            }
                        }
                    }
                    Err(e) => {
                        monitor.report_error(StreamError::SegmentWorker(SegmentWorkerError::StateError(e.to_string())));
                        return None;
                    }
                }
            }
            recv(cancel_r) -> _ => {
                warn!("[DECRYPT SEGMENT] cancelled, exiting");
                return None;
            }
            default(std::time::Duration::from_millis(10)) => {
                if monitor.is_cancelled() {
                    warn!("[DECRYPT SEGMENT] cancelled (timeout path), exiting");
                    return None;
                }
            }
        }
    }

    if (data_frames.len() + 2) != frame_count {
        monitor.report_error(StreamError::SegmentWorker(SegmentWorkerError::InvalidSegment(
            format!("Expected {} frames, got {}", frame_count, data_frames.len() + 2),
        )));
        return None;
    }

    let data_frame_count = data_frames.len() as u32;
    let segment_index = data_frames.first().map(|f| f.segment_index).unwrap_or(input.header.segment_index());

    // ---- Stage 5: Verify digest ----
    let start = Instant::now();
    let digest_frame_data = match digest_frame {
        Some(f) => f,
        None => {
            monitor.report_error(StreamError::SegmentWorker(SegmentWorkerError::MissingDigestFrame));
            return None;
        }
    };

    if digest_frame_data.frame_index != data_frame_count {
        monitor.report_error(StreamError::SegmentWorker(SegmentWorkerError::InvalidSegment(
            "Digest frame at incorrect position".into(),
        )));
        return None;
    }

    let digest_frame_payload = match DigestFrame::decode(&digest_frame_data.plaintext) {
        Ok(df) => df,
        Err(e) => {
            monitor.report_error(StreamError::SegmentWorker(SegmentWorkerError::DigestError(e)));
            return None;
        }
    };

    for frame in &data_frames {
        counters.bytes_overhead += FrameHeader::LEN as u64;
        counters.bytes_compressed += frame.plaintext.len() as u64;
    }
    counters.frames_data = data_frame_count as u64;
    
    if let Err(e) = SegmentDigestVerifier::verify(digest_actual, digest_frame_payload.digest) {
        monitor.report_error(StreamError::SegmentWorker(SegmentWorkerError::DigestError(e)));
        return None;
    }

    counters.add_digest(digest_frame_data.plaintext.len());
    stage_times.add(Stage::Digest, start.elapsed());

    // ---- Stage 6: Terminator ----
    let start = Instant::now();
    let terminator_frame_data = match terminator_frame {
        Some(f) => f,
        None => {
            monitor.report_error(StreamError::SegmentWorker(SegmentWorkerError::MissingTerminatorFrame));
            return None;
        }
    };

    if terminator_frame_data.frame_index != data_frame_count + 1 {
        monitor.report_error(StreamError::SegmentWorker(SegmentWorkerError::InvalidSegment(
            "Terminator frame must be last".into(),
        )));
        return None;
    }

    counters.add_terminator(terminator_frame_data.plaintext.len());
    stage_times.add(Stage::Validate, start.elapsed());

    // ---- Stage 7: Reassemble plaintext ----
    let start = Instant::now();
    let total_plaintext_len: usize = data_frames.iter().map(|f| f.plaintext.len()).sum();
    let mut plaintext_out = Vec::with_capacity(total_plaintext_len);
    for frame in data_frames {
        plaintext_out.extend_from_slice(&frame.plaintext);
    }
    let bytes = Bytes::from(plaintext_out);

    stage_times.add(Stage::Write, start.elapsed());

    debug!(
        "[DECRYPT SEGMENT] completed segment {} ({} bytes plaintext from {} frames)",
        segment_index,
        bytes.len(),
        data_frame_count
    );

    monitor.report_telemetry(TelemetryEvent::StageSnapshot {
        stage_times: stage_times.clone(),
        counters: counters.clone(),
    });

    Some(DecryptedSegment {
        header: input.header,
        bytes,
        counters,
        stage_times,
    })
}
