use std::{sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
}, time::Instant};
use bytes::Bytes;
use crossbeam::channel::{Receiver, Sender};
use tracing::debug;

use crate::{crypto::{DigestAlg, DigestFrame, SegmentDigestBuilder}, stream_v2::{frame_worker::{EncryptedFrame, FrameInput, FrameWorkerError}, framing::FrameType, segment_worker::{EncryptSegmentInput, EncryptedSegment, SegmentWorkerError}, segmenting::{SegmentHeader, types::SegmentFlags}}, telemetry::{Stage, StageTimes, TelemetryCounters}, utils::tracing_logger};

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
pub fn process_encrypt_segment_1(
    input: &EncryptSegmentInput,
    frame_size: usize,
    digest_alg: DigestAlg,
    frame_tx: &Sender<FrameInput>,
    out_rx: &Receiver<Result<EncryptedFrame, FrameWorkerError>>,
    cancelled: Arc<AtomicBool>,
) -> Result<EncryptedSegment, SegmentWorkerError> {
    // explicitly set DEBUG level
    tracing_logger(Some(tracing::Level::DEBUG));

    let mut counters = TelemetryCounters::default();
    let mut stage_times = StageTimes::default();

    debug!(
        "[ENCRYPT SEGMENT] processing segment {}",
        input.segment_index
    );

    // ---- Stage 1: Validation ----
    let start = Instant::now();

    // Handle empty final segment (EOF marker)
    if input.bytes.is_empty() && input.flags.contains(SegmentFlags::FINAL_SEGMENT) {
        debug!(
            "[ENCRYPT SEGMENT] empty FINAL_SEGMENT at index {}",
            input.segment_index
        );
        let header = SegmentHeader::new(
            &Bytes::new(),
            input.segment_index,
            0, // no bytes
            0, // no frames
            digest_alg as u16,
            input.flags,
        );
        return Ok(EncryptedSegment {
            header,
            wire: Bytes::new(),
            counters,
            stage_times,
        });
    }

    // Count segment header overhead
    counters.add_header(SegmentHeader::LEN);

    // Calculate frame count
    let bytes_len = input.bytes.len();
    let frame_count = (bytes_len + frame_size - 1) / frame_size;
    if frame_count == 0 {
        return Err(SegmentWorkerError::InvalidSegment(
            "Empty segment without FINAL_SEGMENT flag".into(),
        ));
    }

    stage_times.add(Stage::Validate, start.elapsed());

    // ---- Stage 2: Dispatch frames for parallel encryption ----
    let start = Instant::now();
    debug!(
        "[ENCRYPT SEGMENT] dispatching {} frames for encryption",
        frame_count
    );

    let all_bytes = &input.bytes; // assume this is already a Bytes
    for (frame_index, chunk) in all_bytes.chunks(frame_size).enumerate() {
        // Compute the offset of this chunk relative to the original buffer
        let start = frame_index * frame_size;
        let end = start + chunk.len();

        frame_tx
            .send(FrameInput {
                segment_index: input.segment_index,
                frame_index: frame_index as u32,
                frame_type: FrameType::Data,
                // Zero-copy slice into the original Bytes
                payload: all_bytes.slice(start..end),
            })
            .map_err(|e| {
                SegmentWorkerError::StateError(e.to_string())
            })?;
    }

    stage_times.add(Stage::Read, start.elapsed());

    // ---- Stage 3: Collect encrypted frames ----
    // let mut data_frames = Vec::with_capacity(frame_count);
    let mut data_frames = vec![EncryptedFrame::default(); frame_count];
    let mut data_wire_len = 0;
    let mut received = 0;

    debug!(
        "[ENCRYPT SEGMENT] collecting {} encrypted frames",
        frame_count
    );

    while received < frame_count {
        // Check for cancellation during collection
        if cancelled.load(Ordering::Relaxed) {
            return Err(SegmentWorkerError::FrameWorkerError(
                FrameWorkerError::WorkerDisconnected,
            ));
        }

        match out_rx.recv() {
            Ok(Ok(frame)) => {
                let idx = frame.frame_index;
                received += 1;
                debug!(
                    "[ENCRYPT SEGMENT] {} received frame {} (type {:?})",
                    frame.segment_index, idx, frame.frame_type
                );

                // Merge frame telemetry
                stage_times.merge(&frame.stage_times);
                // data_frames.push(frame);
                data_frames[idx as usize] = frame;
            }
            Ok(Err(e)) => {
                // Frame worker returned an error
                return Err(e.into());
            }
            Err(e) => {
                // Frame output channel closed unexpectedly
                return Err(SegmentWorkerError::StateError(e.to_string()));
            }
        }
    }

    // Ensure we received exactly the expected number of frames
    if data_frames.len() != frame_count {
        return Err(SegmentWorkerError::InvalidSegment(
            format!(
                "Frame count mismatch: expected {}, received {}",
                frame_count,
                data_frames.len()
            ),
        ));
    }

    // ---- Stage 4: Compute segment digest ----
    let start = Instant::now();
    let mut digest_builder =
        SegmentDigestBuilder::new(digest_alg, input.segment_index, frame_count as u32);

    for frame in &data_frames {
        data_wire_len += frame.wire.len();

        // Track overhead: frame header
        counters.bytes_overhead += EncryptedFrame::frame_overhead() as u64;
        // Track ciphertext size
        counters.bytes_ciphertext += frame.ciphertext().len() as u64;

        // Update digest with frame ciphertext
        digest_builder.update_frame(frame.frame_index, frame.ciphertext());
    }

    counters.frames_data = frame_count as u64;

    // Finalize digest
    let digest = digest_builder.finalize()?;
    let digest_payload = Bytes::from(DigestFrame::new(digest_alg, digest).encode());

    // ---- Stage 5: Create digest frame ----
    frame_tx
        .send(FrameInput {
            segment_index: input.segment_index,
            frame_index: frame_count as u32,
            frame_type: FrameType::Digest,
            payload: digest_payload,
        })
        .map_err(|e| {
            SegmentWorkerError::StateError(e.to_string())
        })?;

    let digest_frame = match out_rx.recv() {
        Ok(Ok(frame)) => frame,
        Ok(Err(e)) => return Err(e.into()),
        Err(e) => return Err(SegmentWorkerError::StateError(e.to_string()))
    };

    debug!(
        "[ENCRYPT SEGMENT] digest frame created for segment {}",
        input.segment_index
    );
    stage_times.add(Stage::Digest, start.elapsed());
    counters.add_digest(digest_frame.ciphertext().len());

    // ---- Stage 6: Create terminator frame ----
    let start = Instant::now();
    frame_tx
        .send(FrameInput {
            segment_index: input.segment_index,
            frame_index: frame_count as u32 + 1,
            frame_type: FrameType::Terminator,
            payload: Bytes::new(),
        })
        .map_err(|e| {
            SegmentWorkerError::StateError(e.to_string())
        })?;

    let terminator_frame = match out_rx.recv() {
        Ok(Ok(frame)) => frame,
        Ok(Err(e)) => return Err(e.into()),
        Err(e) => return Err(SegmentWorkerError::StateError(e.to_string())),
    };

    debug!(
        "[ENCRYPT SEGMENT] terminator frame created for segment {}",
        input.segment_index
    );
    counters.add_terminator(terminator_frame.ciphertext().len());
    stage_times.add(Stage::Validate, start.elapsed());

    // ---- Stage 7: Serialize all frames into wire format ----
    let start = Instant::now();
    let total_len = data_wire_len + digest_frame.wire.len() + terminator_frame.wire.len();
    let mut wire_bytes = Vec::with_capacity(total_len);

    // Concatenate: data frames + digest frame + terminator frame
    for frame in data_frames {
        wire_bytes.extend_from_slice(&frame.wire);
    }
    wire_bytes.extend_from_slice(&digest_frame.wire);
    wire_bytes.extend_from_slice(&terminator_frame.wire);

    let wire = Bytes::from(wire_bytes);

    // Create segment header
    let header = SegmentHeader::new(
        &wire,
        input.segment_index,
        bytes_len as u32,
        frame_count as u32,
        digest_alg as u16,
        input.flags,
    );

    stage_times.add(Stage::Write, start.elapsed());

    debug!(
        "[ENCRYPT SEGMENT] completed segment {} ({} bytes -> {} frames)",
        input.segment_index, bytes_len, frame_count
    );

    Ok(EncryptedSegment {
        header,
        wire,
        counters,
        stage_times,
    })
}
