// # 📂 `src/stream_v3/segment_worker/encrypt.rs`

use std::sync::Arc;
use crossbeam::channel::{Receiver, Sender, bounded, unbounded};
use tracing::{error, info, warn};

use crate::{
    recovery::AsyncLogManager, 
    stream::{frame_worker::{EncryptedFrame, FrameInput}, segment_worker::{EncryptContext, EncryptedSegment, SegmentWorkerError}},
    types::StreamError, utils::tracing_logger
};

use crate::v3::stream::{
    frame_worker::EncryptFrameWorker3, pipeline::{Monitor, PipelineMonitor}, 
    segment_worker::{SegmentInput, enc_helpers::process_encrypt_segment_3}
};

#[derive(Debug, Clone)]
pub struct EncryptSegmentWorker3 {
    crypto: Arc<EncryptContext>,          // shared immutable context
    log_manager: Arc<AsyncLogManager>,
    monitor: Monitor,                     // global monitor for errors + telemetry
}

impl EncryptSegmentWorker3 {
    pub fn new(
        crypto: Arc<EncryptContext>,
        log_manager: Arc<AsyncLogManager>,
        monitor: Monitor,
    ) -> Self {
        Self { crypto, log_manager, monitor }
    }

    /// Runs the segment worker loop with an internal frame worker pool
    pub fn run(
        self,
        rx: Receiver<SegmentInput>,
        tx: Sender<EncryptedSegment>,   // 👈 plain segment, no Result
    ) {
        tracing_logger(Some(tracing::Level::DEBUG));

        let crypto = self.crypto.clone();
        let monitor = self.monitor.clone();

        // ---- Initialize frame worker pool ----
        let worker_count = crypto.base.profile.cpu_workers();
        let digest_alg = crypto.base.digest_alg;
        let frame_size = crypto.base.frame_size;

        let (frame_tx, frame_rx) = bounded::<FrameInput>(worker_count * 4);
        let (out_tx, out_rx) = unbounded::<EncryptedFrame>();

        for _ in 0..worker_count {
            if monitor.is_cancelled() {
                warn!("[ENCRYPT SEGMENT WORKER] cancelled, exiting early");
                break;
            }

            let fw = EncryptFrameWorker3::new(
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
                warn!("[ENCRYPT SEGMENT WORKER] cancelled, exiting early");
                break;
            }

            let segment_idx = segment.index;
            info!("[ENCRYPT SEGMENT WORKER] processing segment {}", segment_idx);

            if let Some(encrypted_segment) = process_encrypt_segment_3(
                &segment,
                frame_size,
                digest_alg,
                &frame_tx,
                &out_rx,
                monitor.clone(),
            ) {
                if let Err(e) = tx.send(encrypted_segment) {
                    error!("[ENCRYPT SEGMENT WORKER] tx send failed: {}", e);
                    monitor.report_error(StreamError::SegmentWorker(
                        SegmentWorkerError::StateError(e.to_string()),
                    ));
                    break;
                }
                self.log_manager.console(
                    format!("[ENCRYPT SEGMENT]: {} successfully encrypted", segment_idx).into()
                );
            } else {
                // Error already reported via monitor
                error!("[ENCRYPT SEGMENT WORKER] segment {} failed", segment_idx);
                break;
            }
        }

        drop(frame_tx);
        drop(tx);
        warn!("[ENCRYPT SEGMENT WORKER] dropped channels, thread exiting");
    }
}
