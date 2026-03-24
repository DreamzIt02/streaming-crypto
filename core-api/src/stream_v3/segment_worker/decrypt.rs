// # 📂 `src/stream_v3/segment_worker/decrypt.rs`

use bytes::Bytes;
use crossbeam::channel::{Receiver, Sender, bounded, unbounded};
use tracing::{debug, error, info, warn};
use std::sync::Arc;

use crate::{
    recovery::AsyncLogManager, 
    stream_v2::{frame_worker::{DecryptedFrame}, segment_worker::{DecryptContext, DecryptedSegment, SegmentWorkerError}},
    stream_v3::{frame_worker::DecryptFrameWorker3, pipeline::{Monitor, PipelineMonitor}, segment_worker::{SegmentInput, dec_helpers::process_decrypt_segment_3}}, 
    types::StreamError, utils::tracing_logger
};

#[derive(Debug, Clone)]
pub struct DecryptSegmentWorker3 {
    crypto: Arc<DecryptContext>,          // shared immutable context
    log_manager: Arc<AsyncLogManager>,
    monitor: Monitor,                     // global monitor for errors + telemetry
}

impl DecryptSegmentWorker3 {
    pub fn new(
        crypto: Arc<DecryptContext>,
        log_manager: Arc<AsyncLogManager>,
        monitor: Monitor,
    ) -> Self {
        Self { crypto, log_manager, monitor }
    }

    /// Runs the segment worker loop with an internal frame worker pool
    pub fn run(
        self,
        rx: Receiver<SegmentInput>,
        tx: Sender<DecryptedSegment>,   // 👈 plain segment, no Result
    ) {
        tracing_logger(Some(tracing::Level::DEBUG));
        debug!("[DECRYPT SEGMENT WORKER] thread started");

        let crypto = self.crypto.clone();
        let monitor = self.monitor.clone();

        // ---- Initialize frame worker pool ----
        let worker_count = crypto.base.profile.cpu_workers();
        let digest_alg = crypto.base.digest_alg;

        let (frame_tx, frame_rx) = bounded::<Bytes>(worker_count * 4);
        let (out_tx, out_rx) = unbounded::<DecryptedFrame>(); // 👈 clean frames only

        for _ in 0..worker_count {
            if monitor.is_cancelled() {
                warn!("[DECRYPT SEGMENT WORKER] cancelled, exiting early");
                break;
            }
            let fw = DecryptFrameWorker3::new(
                crypto.header.clone(),
                &crypto.base.session_key,
                monitor.clone(),
            );

            let rx_clone = frame_rx.clone();
            let tx_clone = out_tx.clone();

            std::thread::spawn(move || {
                fw.run(rx_clone, tx_clone);
            });
        }

        drop(frame_rx);
        drop(out_tx);

        // ---- Process segments sequentially ----
        while let Ok(segment) = rx.recv() {
            if monitor.is_cancelled() {
                warn!("[DECRYPT SEGMENT WORKER] cancelled, exiting early");
                break;
            }

            // let (frame_tx, frame_rx) = bounded::<Bytes>(worker_count * 4);
            // let (out_tx, out_rx) = unbounded::<DecryptedFrame>();

            // // spawn frame workers for this segment
            // for _ in 0..worker_count {
            //     if monitor.is_cancelled() {
            //         warn!("[DECRYPT SEGMENT WORKER] cancelled, exiting early");
            //         break;
            //     }
            //     let fw = DecryptFrameWorker3::new(crypto.header.clone(), &crypto.base.session_key, monitor.clone());
            //     let rx_clone = frame_rx.clone();
            //     let tx_clone = out_tx.clone();
            //     std::thread::spawn(move || fw.run(rx_clone, tx_clone));
            // }
            // drop(frame_rx);
            // drop(out_tx);

            let segment_idx = segment.header.segment_index();
            info!("[DECRYPT SEGMENT WORKER] processing segment {}", segment_idx);

            if let Some(decrypted_segment) = process_decrypt_segment_3(
                &segment,
                &digest_alg,
                &frame_tx,
                &out_rx,
                monitor.clone(),
            ) {
                if let Err(e) = tx.send(decrypted_segment) {
                    error!("[DECRYPT SEGMENT WORKER] tx send failed: {}", e);
                    monitor.report_error(StreamError::SegmentWorker(
                        SegmentWorkerError::StateError(e.to_string()),
                    ));
                    break;
                }
                self.log_manager.console(
                    format!("[DECRYPT SEGMENT]: {} successfully decrypted", segment_idx).into()
                );
            } else {
                // Error already reported via monitor
                error!("[DECRYPT SEGMENT WORKER] segment {} failed", segment_idx);
                break;
            }

        }
        warn!("[DECRYPT SEGMENT WORKER] dropping frame_tx and exiting");
        drop(frame_tx);
        drop(tx);
    }
}
