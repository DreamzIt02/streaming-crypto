// # 📂 `src/stream_v2/segment_worker/decrypt.rs`

use bytes::Bytes;
use crossbeam::channel::{Receiver, Sender, bounded, unbounded};
use tracing::{debug, error};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};


use core_api::{
    recovery::AsyncLogManager,
    stream::{
        frame_worker::{DecryptedFrame, FrameWorkerError},
        segment_worker::{DecryptContext, SegmentWorkerError, types::{DecryptSegmentInput, DecryptedSegment}},
    }, 
    types::StreamError, utils::tracing_logger
};

use crate::frame_worker::decrypt::DecryptFrameWorker1;
use super::{dec_helpers::process_decrypt_segment_1};

#[derive(Debug, Clone)]
pub struct DecryptSegmentWorker1 {
    crypto: Arc<DecryptContext>,                 // shared immutable context
    log_manager: Arc<AsyncLogManager>,
    fatal_tx: Sender<StreamError>,               // global error channel
    cancelled: Arc<AtomicBool>,                  // global cancellation flag
}

impl DecryptSegmentWorker1 {
    /// Creates a new segment decryption worker
    ///
    /// # Arguments
    /// * `crypto` - Shared decryption context containing keys and configuration
    /// * `log_manager` - Async logging manager for audit trails
    /// * `fatal_tx` - Channel to signal fatal errors to the pipeline monitor
    /// * `cancelled` - Shared cancellation flag for graceful shutdown
    pub fn new(
        crypto: Arc<DecryptContext>,
        log_manager: Arc<AsyncLogManager>,
        fatal_tx: Sender<StreamError>,
        cancelled: Arc<AtomicBool>,
    ) -> Self {
        Self {
            crypto,
            log_manager,
            fatal_tx,
            cancelled,
        }
    }

    /// Runs the segment worker loop with an internal frame worker pool
    ///
    /// # Architecture
    /// - Spawns a pool of frame workers for parallel frame decryption
    /// - Processes segments sequentially, frames within segments in parallel
    /// - Each segment wire is split into frames, decrypted, verified, and reassembled
    /// - Coordinates with frame workers via bounded channels
    ///
    /// # Behavior
    /// - Validates segment headers before processing
    /// - Checks cancellation before processing each segment
    /// - Propagates errors via fatal_tx to trigger pipeline shutdown
    /// - Exits gracefully when input channel closes or on cancellation
    pub fn run_v1(
        self,
        rx: Receiver<DecryptSegmentInput>,
        tx: Sender<Result<DecryptedSegment, SegmentWorkerError>>,
    ) {
        // explicitly set DEBUG level
        tracing_logger(Some(tracing::Level::DEBUG));

        let crypto = self.crypto.clone();
        let fatal_tx = self.fatal_tx.clone();
        let cancelled = self.cancelled.clone();

        // Remove thread::spawn - we're already in a scoped thread!
            debug!("[DECRYPT SEGMENT WORKER] thread spawned");

            // ---- Initialize frame worker pool ----
            let worker_count = crypto.base.profile.cpu_workers();
            let digest_alg = crypto.base.digest_alg;

            // Frame processing channels
            let (frame_tx, frame_rx) = bounded::<Bytes>(worker_count * 4);
            let (out_tx, out_rx) = unbounded::<Result<DecryptedFrame, FrameWorkerError>>();

            // Spawn frame decryption workers
            for _ in 0..worker_count {
                let fw = DecryptFrameWorker1::new(
                    crypto.header.clone(),
                    &crypto.base.session_key,
                    fatal_tx.clone(),
                    cancelled.clone(),
                )
                .expect("DecryptFrameWorker pool initialization failed");

                let rx_clone = frame_rx.clone();
                let tx_clone = out_tx.clone();
                
                // Spawn in a new thread - frame workers need to run concurrently
                std::thread::spawn(move || {
                    fw.run(rx_clone, tx_clone);
                });
            }

            // Drop original senders so workers know when to exit
            drop(frame_rx);
            drop(out_tx);

            // ---- Process segments sequentially ----
            loop {
                // Check for cancellation before blocking on receive
                if cancelled.load(Ordering::Relaxed) {
                    error!("[DECRYPT SEGMENT WORKER] cancelled, exiting early");
                    break;
                }

                // Receive next segment (blocks until available or channel closes)
                let segment = match rx.recv() {
                    Ok(segment) => segment,
                    Err(_) => {
                        // Channel closed normally - all segments processed
                        debug!("[DECRYPT SEGMENT WORKER] rx closed, exiting");
                        break;
                    }
                };

                let segment_idx = segment.header.segment_index();
                debug!(
                    "[DECRYPT SEGMENT WORKER] processing segment {}",
                    segment_idx
                );
                // Validate segment header before processing
                match segment.header.validate(&segment.wire) {
                    Ok(()) => {
                        // Process the segment (splits frames, decrypts, verifies digest)
                        let result = process_decrypt_segment_1(
                            &segment,
                            &digest_alg,
                            &frame_tx,
                            &out_rx,
                            cancelled.clone(),
                        );

                        match result {
                            Ok(decrypted_segment) => {
                                // Send decrypted segment to output
                                if let Err(e) = tx.send(Ok(decrypted_segment)) {
                                    // Output channel closed unexpectedly - pipeline is shutting down
                                    error!(
                                        "[DECRYPT SEGMENT WORKER] tx send failed, receiver disconnected"
                                    );
                                    let _ = fatal_tx.send(StreamError::SegmentWorker(
                                        SegmentWorkerError::StateError(e.to_string()),
                                    ));
                                    cancelled.store(true, Ordering::Relaxed);
                                    break;
                                }
                                // Append log for successfully decrypted segment
                                self.log_manager.console(("[DECRYPT SEGMENT]: ".to_string() + &segment_idx.to_string() + " successfully decrypted").into());
                            }
                            Err(e) => {
                                // Segment processing failed - this is a fatal error
                                // This prints raw arrays (Debug)
                                // error!("[DECRYPT SEGMENT WORKER] processing error: {:?}", e);

                                // This prints hex strings (Display)
                                error!("[DECRYPT SEGMENT WORKER] processing error: {}", e);

                                // Signal fatal error to pipeline monitor
                                let _ = fatal_tx.send(StreamError::SegmentWorker(e.clone()));

                                // Set cancellation flag to stop other workers
                                cancelled.store(true, Ordering::Relaxed);

                                // Try to send error to output (best effort)
                                let _ = tx.send(Err(e));
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        // Header validation failed - this is a fatal error
                        error!(
                            "[DECRYPT SEGMENT WORKER] header validation failed: {:?}",
                            e
                        );

                        // Try to send error to output (best effort)
                        let _ = tx.send(Err(SegmentWorkerError::SegmentError(e.clone())));

                        // Signal fatal error to pipeline monitor
                        let _ = fatal_tx.send(StreamError::Segment(e));

                        // Set cancellation flag to stop other workers
                        cancelled.store(true, Ordering::Relaxed);
                        break;
                    }
                }
            }

            // Cleanup: drop channels to signal frame workers to exit
            debug!("[DECRYPT SEGMENT WORKER] dropping frame_tx and exiting");
            drop(frame_tx);
            drop(tx);

    }
}

