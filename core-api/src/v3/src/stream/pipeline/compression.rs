// ## 📦 `src/stream_v3/pipeline/compression.rs`

use std::sync::{Arc, Mutex};
use crossbeam::channel::{Receiver, Sender};
use crossbeam::thread::Scope;
use tracing::{info, warn};

use crate::{
    stream::{compression_worker::{CodecInfo, make_backend}, segment_worker::DecryptedSegment},
    parallelism::{HybridParallelismProfile, Scheduler, WorkerTarget},
};

use crate::v3::stream::{
    compression_worker::{run_compress_worker, run_decompress_worker}, 
    pipeline::Monitor, 
    segment_worker::SegmentInput,
    pipeline::PipelineMonitor,
};

/// Compression worker entry point
pub fn spawn_compress_workers_scoped<'scope>(
    scope       : &Scope<'scope>,
    profile     : HybridParallelismProfile,
    codec_info  : CodecInfo,
    comp_rx     : Receiver<SegmentInput>,
    out_tx      : Sender<SegmentInput>,   // 👈 plain segment, no Result
    monitor     : Monitor,
) {
    let workers = (profile.cpu_workers() / 2).max(1);

    let scheduler = Arc::new(Mutex::new(Scheduler::new(
        profile.cpu_workers(),
        profile.gpu_workers(),
        profile.gpu_threshold(),
    )));

    // CPU workers
    for worker_id in 0..workers {
        if monitor.is_cancelled() {
            warn!("[SPAWN COMPRESS] cancelled, exiting early");
            break;
        }
        let backend = make_backend(WorkerTarget::Cpu(worker_id), codec_info.clone());
        let sched = scheduler.clone();
        let rx = comp_rx.clone();
        let tx = out_tx.clone();
        let monitor = monitor.clone();

        scope.spawn(move |_| {
            let thread = std::thread::current();
            info!(
                "[SPAWN COMPRESS] started: worker_id={}, thread_id={:?}, name={:?}",
                worker_id,
                thread.id(),
                thread.name()
            );

            run_compress_worker(rx, tx, backend, sched, monitor);
            warn!(
                "[SPAWN COMPRESS] exiting: worker_id={}, thread_id={:?}, name={:?}",
                worker_id,
                thread.id(),
                thread.name()
            );
        });
    }

    // GPU workers
    for i in 0..profile.gpu_workers() {
        if monitor.is_cancelled() {
            warn!("[SPAWN COMPRESS] cancelled, exiting early");
            break;
        }
        let backend = make_backend(WorkerTarget::Gpu(i), codec_info.clone());
        let sched = scheduler.clone();
        let rx = comp_rx.clone();
        let tx = out_tx.clone();
        let monitor = monitor.clone();

        scope.spawn(move |_| {
            run_compress_worker(rx, tx, backend, sched, monitor);
        });
    }
    drop(comp_rx);
    drop(out_tx);
}

/// Decompression worker entry point
pub fn spawn_decompress_workers_scoped<'scope>(
    scope       : &Scope<'scope>,
    profile     : HybridParallelismProfile,
    codec_info  : CodecInfo,
    comp_rx     : Receiver<DecryptedSegment>,
    out_tx      : Sender<DecryptedSegment>,   // 👈 plain segment, no Result
    monitor     : Monitor,
) {
    let scheduler = Arc::new(Mutex::new(Scheduler::new(
        profile.cpu_workers(),
        profile.gpu_workers(),
        profile.gpu_threshold(),
    )));
    let workers = (profile.cpu_workers() / 2).max(1);

    // CPU workers
    for i in 0..workers {
        if monitor.is_cancelled() {
            warn!("[SPAWN DECOMPRESS] cancelled, exiting early");
            break;
        }
        let backend = make_backend(WorkerTarget::Cpu(i), codec_info.clone());
        let sched = scheduler.clone();
        let rx = comp_rx.clone();
        let tx = out_tx.clone();
        let monitor = monitor.clone();

        scope.spawn(move |_| {
            run_decompress_worker(rx, tx, backend, sched, monitor);
        });
    }

    // GPU workers
    for i in 0..profile.gpu_workers() {
        if monitor.is_cancelled() {
            warn!("[SPAWN DECOMPRESS] cancelled, exiting early");
            break;
        }
        let backend = make_backend(WorkerTarget::Gpu(i), codec_info.clone());
        let sched = scheduler.clone();
        let rx = comp_rx.clone();
        let tx = out_tx.clone();
        let monitor = monitor.clone();

        scope.spawn(move |_| {
            run_decompress_worker(rx, tx, backend, sched, monitor);
        });
    }
    drop(comp_rx);
    drop(out_tx);
}
