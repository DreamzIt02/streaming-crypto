// ## 📦 `src/stream_v2/segment_worker/dec_helpers.rs`

use std::{sync::{Arc, atomic::{AtomicBool, Ordering}}, time::Instant};

use bytes::Bytes;
use crossbeam::channel::{Receiver, Sender};
use tracing::{debug, error};

use crate::{crypto::{DigestAlg, DigestFrame, SegmentDigestVerifier}, stream_v2::{frame_worker::{DecryptedFrame, FrameWorkerError}, framing::{FrameError, FrameHeader, FrameType}, segment_worker::{DecryptSegmentInput, DecryptedSegment, SegmentWorkerError}, segmenting::{SegmentHeader, types::SegmentFlags}}, telemetry::{Stage, StageTimes, TelemetryCounters}, utils::tracing_logger};

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

pub fn process_decrypt_segment_1(
    input: &DecryptSegmentInput,
    digest_alg: &DigestAlg,
    frame_tx: &Sender<Bytes>,
    out_rx: &Receiver<Result<DecryptedFrame, FrameWorkerError>>,
    cancelled: Arc<AtomicBool>,
) -> Result<DecryptedSegment, SegmentWorkerError> {
    // explicitly set DEBUG level
    tracing_logger(Some(tracing::Level::DEBUG));

    let mut counters = TelemetryCounters::default();
    let mut stage_times = StageTimes::default();

    debug!(
        "[DECRYPT SEGMENT] processing segment {}",
        input.header.segment_index()
    );

    // ---- Stage 1: Validation ----
    let start = Instant::now();

    // Handle empty final segment (EOF marker)
    if input.wire.is_empty() && input.header.flags().contains(SegmentFlags::FINAL_SEGMENT) {
        debug!(
            "[DECRYPT SEGMENT] empty FINAL_SEGMENT at index {}",
            input.header.segment_index()
        );
        return Ok(DecryptedSegment {
            header: input.header.clone(),
            bytes: Bytes::new(),
            counters,
            stage_times,
        });
    }

    // Verify CRC32 checksum of segment wire
    input
        .header
        .validate(&input.wire)
        .map_err(SegmentWorkerError::SegmentError)?;

    stage_times.add(Stage::Validate, start.elapsed());

    // Count segment header overhead
    counters.add_header(SegmentHeader::LEN);

    // ---- Stage 2: Parse frame boundaries and dispatch for decryption ----
    let start = Instant::now();
    let mut offset = 0;
    let mut frame_count: usize = 0;

    debug!(
        "[DECRYPT SEGMENT] parsing frames from wire (length: {} bytes)",
        input.wire.len()
    );

    // Initialize digest verifier
    let mut verifier = SegmentDigestVerifier::new(
        digest_alg.clone(),
        input.header.segment_index(),
        input.header.frame_count(),
    );

    while offset < input.wire.len() {
        // Parse frame header to determine frame length
        let header = FrameHeader::from_bytes(&input.wire[offset..])?;
        let frame_len = FrameHeader::LEN + header.ciphertext_len() as usize;
        let end = offset + frame_len;

        // Validate frame doesn't extend beyond wire boundary
        if end > input.wire.len() {
            error!("[DECRYPT SEGMENT] frame truncated at offset {}", offset);
            return Err(FrameError::Truncated.into());
        }

        debug!(
            "[DECRYPT SEGMENT] dispatching frame {} (segment {}, length: {} bytes)",
            frame_count,
            input.header.segment_index(),
            frame_len
        );
        // ✅ Update digest first, using ciphertext slice
        if header.frame_type() == FrameType::Data {
            let ct_start = offset + FrameHeader::LEN;
            let ct_end = ct_start + header.ciphertext_len() as usize;

            verifier.update_frame(header.frame_index(), &input.wire[ct_start..ct_end]);
        }

        // Dispatch frame slice for decryption (zero-copy using Bytes::slice)
        frame_tx
            .send(input.wire.slice(offset..end))
            .map_err(|e| {
                SegmentWorkerError::StateError(e.to_string())
            })?;

        offset = end;
        frame_count += 1;
    }
    //
    let digest_actual = verifier.finalize()?;
    
    stage_times.add(Stage::Read, start.elapsed());

    // Validate we found at least one frame
    if frame_count == 0 {
        error!(
            "[DECRYPT SEGMENT] no frames found in non-final segment {}",
            input.header.segment_index()
        );
        return Err(SegmentWorkerError::InvalidSegment(
            "Segment contains no frames".into(),
        ));
    }

    // ---- Stage 3: Collect decrypted frames ----
    // let mut data_frames = Vec::with_capacity(frame_count.saturating_sub(2));
    let mut data_frames =  vec![DecryptedFrame::default(); frame_count.saturating_sub(2)];
    let mut digest_frame: Option<DecryptedFrame> = None;
    let mut terminator_frame: Option<DecryptedFrame> = None;
    let mut received = 0;

    debug!(
        "[DECRYPT SEGMENT] collecting {} decrypted frames",
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
                    "[DECRYPT SEGMENT] {} received frame {} (type {:?})",
                    frame.segment_index, frame.frame_index, frame.frame_type
                );

                // Merge frame telemetry
                stage_times.merge(&frame.stage_times);

                // Categorize frame by type
                match frame.frame_type {
                    FrameType::Data => data_frames[idx as usize] = frame, //data_frames.push(frame),
                    FrameType::Digest => {
                        if digest_frame.is_some() {
                            return Err(SegmentWorkerError::InvalidSegment(
                                "Multiple digest frames detected".into(),
                            ));
                        }
                        digest_frame = Some(frame);
                    }
                    FrameType::Terminator => {
                        if terminator_frame.is_some() {
                            return Err(SegmentWorkerError::InvalidSegment(
                                "Multiple terminator frames detected".into(),
                            ));
                        }
                        terminator_frame = Some(frame);
                    }
                }
            }
            Ok(Err(e)) => {
                // Frame worker returned an error
                error!("[DECRYPT SEGMENT] frame worker error: {:?}", e);
                return Err(e.into());
            }
            Err(e) => {
                // Frame output channel closed unexpectedly
                error!("[DECRYPT SEGMENT] frame worker channel disconnected");
                return Err(SegmentWorkerError::StateError(e.to_string()));
            }
        }
    }

    // Validate frame counts (data frames + digest + terminator)
    if (data_frames.len() + 2) != frame_count {
        error!(
            "[DECRYPT SEGMENT] frame count mismatch: data={}, total={}",
            data_frames.len(),
            frame_count
        );
        return Err(SegmentWorkerError::InvalidSegment(
            format!(
                "Expected {} frames (data+digest+terminator), got {}+2",
                frame_count,
                data_frames.len()
            ),
        ));
    }

    let data_frame_count = data_frames.len() as u32;
    let segment_index = data_frames
        .first()
        .map(|f| f.segment_index)
        .unwrap_or(input.header.segment_index());

    // ---- Stage 5: Verify segment digest ----
    let start = Instant::now();

    let digest_frame_data = digest_frame.ok_or(SegmentWorkerError::MissingDigestFrame)?;

    // Validate digest frame is at expected position
    if digest_frame_data.frame_index != data_frame_count {
        error!(
            "[DECRYPT SEGMENT] digest frame index mismatch: expected {}, got {}",
            data_frame_count, digest_frame_data.frame_index
        );
        return Err(SegmentWorkerError::InvalidSegment(
            "Digest frame at incorrect position".into(),
        ));
    }

    // Decode digest payload
    let digest_frame_payload = DigestFrame::decode(&digest_frame_data.plaintext)?;
    
    debug!(
        "[DECRYPT SEGMENT] digest frame decoded, verifying segment {}",
        segment_index
    );

    // Update verifier with all frame ciphertexts
    for frame in &data_frames {
        // Track overhead: frame header
        counters.bytes_overhead += FrameHeader::LEN as u64;
        // Track compressed/plaintext size
        counters.bytes_compressed += frame.plaintext.len() as u64;
    }

    counters.frames_data = data_frame_count as u64;

    // Finalize and verify digest (fails if mismatch)
    SegmentDigestVerifier::verify(digest_actual, digest_frame_payload.digest)?;

    counters.add_digest(digest_frame_data.plaintext.len());

    stage_times.add(Stage::Digest, start.elapsed());
    debug!(
        "[DECRYPT SEGMENT] digest verified for segment {}",
        segment_index
    );

    // ---- Stage 6: Validate terminator frame ----
    let start = Instant::now();

    let terminator_frame_data =
        terminator_frame.ok_or(SegmentWorkerError::MissingTerminatorFrame)?;

    // Validate terminator frame is at expected position (last frame)
    if terminator_frame_data.frame_index != data_frame_count + 1 {
        error!(
            "[DECRYPT SEGMENT] terminator frame index mismatch: expected {}, got {}",
            data_frame_count + 1,
            terminator_frame_data.frame_index
        );
        return Err(SegmentWorkerError::InvalidSegment(
            "Terminator frame must be the last frame".into(),
        ));
    }

    counters.add_terminator(terminator_frame_data.plaintext.len());
    debug!(
        "[DECRYPT SEGMENT] terminator frame validated for segment {}",
        segment_index
    );

    stage_times.add(Stage::Validate, start.elapsed());

    // ---- Stage 7: Reassemble plaintext ----
    let start = Instant::now();

    // Preallocate buffer for all plaintext
    let total_plaintext_len: usize = data_frames.iter().map(|f| f.plaintext.len()).sum();
    let mut plaintext_out = Vec::with_capacity(total_plaintext_len);

    // Concatenate all data frame plaintext in order
    for frame in data_frames {
        plaintext_out.extend_from_slice(&frame.plaintext);
    }

    let bytes = Bytes::from(plaintext_out);

    // Note: We can validate that header.bytes_len == bytes.len()
    // to ensure plaintext length matches header expectation

    stage_times.add(Stage::Write, start.elapsed());

    debug!(
        "[DECRYPT SEGMENT] completed segment {} ({} bytes plaintext from {} frames)",
        segment_index,
        bytes.len(),
        data_frame_count
    );

    Ok(DecryptedSegment {
        header: input.header,
        bytes,
        counters,
        stage_times,
    })
}

