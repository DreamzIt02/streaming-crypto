// ## 📦 `src/stream_v2/compression_worker/worker.rs`

use std::{sync::{Arc, Mutex}, time::Instant};
use bytes::Bytes;
use crossbeam::channel::{Receiver, Sender};
use tracing::{debug, error};

use core_api::{
    compression_worker::CompressionBackend, parallelism::Scheduler, stream::{
        compression_worker::types::CompressionWorkerError, segment_worker::{DecryptedSegment, EncryptSegmentInput}, segmenting::types::SegmentFlags
    }, telemetry::{Stage, StageTimes}, utils::tracing_logger
};

/// Single compression worker loop
pub fn run_compression_worker(
    rx: Receiver<EncryptSegmentInput>,
    tx: Sender<Result<EncryptSegmentInput, CompressionWorkerError>>,
    mut backend: Box<dyn CompressionBackend>,
    scheduler: Arc<Mutex<Scheduler>>,
) {
    // explicitly set DEBUG level
    tracing_logger(Some(tracing::Level::DEBUG));

    debug!("[COMPRESSION WORKER] started, blocking on rx.recv()");

    while let Ok(mut seg) = rx.recv() {
        debug!("[COMPRESSION WORKER] received segment {} at {:?}", 
                  seg.segment_index, std::time::Instant::now());
                  
        let mut stage_times = StageTimes::default();
        // Compression / segment
        let start = Instant::now();

        let target = {
            let mut sched = scheduler.lock().unwrap();
            sched.dispatch(seg.bytes.len())
        };

        // ✅ Catch final empty segment before compression
        if seg.flags.contains(SegmentFlags::FINAL_SEGMENT) && seg.bytes.is_empty() {
            debug!("[COMPRESSION] final empty segment {} bypassed", seg.segment_index);
            stage_times.add(Stage::Compress, start.elapsed());
            seg.stage_times = stage_times;

            let _ = tx.send(Ok(seg));
            let mut sched = scheduler.lock().unwrap();
            sched.complete(target);
            continue;
        }

        match backend.compress_chunk(&seg.bytes) {
            Ok(buf) => {
                seg.bytes = Bytes::from(buf);
                stage_times.add(Stage::Compress, start.elapsed());
                seg.stage_times = stage_times;

                let _ = tx.send(Ok(seg));
            }
            Err(e) => {
                error!("[COMPRESSION] failed: {e}");
                let _ = tx.send(Err(CompressionWorkerError::Compression(e)));
                break; // exit on error so pipeline can terminate
            }
        }

        let mut sched = scheduler.lock().unwrap();
        sched.complete(target);
    }

    debug!("[COMPRESSION WORKER] rx.recv() returned Err (channel closed), exiting");
    drop(tx); // Explicit drop for clarity
    debug!("[COMPRESSION WORKER] tx dropped");

}

/// Single decompression worker loop
pub fn run_decompression_worker(
    rx: Receiver<DecryptedSegment>,
    tx: Sender<Result<DecryptedSegment, CompressionWorkerError>>,
    mut backend: Box<dyn CompressionBackend>,
    scheduler: Arc<Mutex<Scheduler>>,
) {
    // explicitly set DEBUG level
    tracing_logger(Some(tracing::Level::DEBUG));

    while let Ok(mut seg) = rx.recv() {
        let mut stage_times = StageTimes::default();
        // Decompression / segment
        let start = Instant::now();

        let target = {
            let mut sched = scheduler.lock().unwrap();
            sched.dispatch(seg.bytes.len())
        };

        // ✅ Catch final empty segment before decompression
        if seg.header.flags().contains(SegmentFlags::FINAL_SEGMENT) && seg.bytes.is_empty() {
            debug!("[DECOMPRESSION] final empty segment {} bypassed", seg.header.segment_index());
            stage_times.add(Stage::Decompress, start.elapsed());
            seg.stage_times = stage_times;

            let _ = tx.send(Ok(seg));
            let mut sched = scheduler.lock().unwrap();
            sched.complete(target);
            continue;
        }
        
        match backend.decompress_chunk(&seg.bytes) {
            Ok(buf) => {
                seg.bytes = Bytes::from(buf);
                stage_times.add(Stage::Decompress, start.elapsed());
                seg.stage_times = stage_times;

                let _ = tx.send(Ok(seg));
            }
            Err(e) => {
                debug!("[DECOMPRESSION] failed: {e}");
                let _ = tx.send(Err(CompressionWorkerError::Compression(e)));
                break; // exit on error so pipeline can terminate
            }
        }

        let mut sched = scheduler.lock().unwrap();
        sched.complete(target);
    }
}

