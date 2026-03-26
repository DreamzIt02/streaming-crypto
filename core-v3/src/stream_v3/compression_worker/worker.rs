// ## 📦 `src/stream_v3/compression_worker/worker.rs`

use std::{sync::{Arc, Mutex}, time::Instant};
use bytes::Bytes;
use crossbeam::channel::{Receiver, Sender};
use tracing::{debug, warn};

use core_api::{
    parallelism::Scheduler, 
    stream_v2::{
        segmenting::types::SegmentFlags, 
        compression_worker::CompressionBackend,
        segment_worker::DecryptedSegment
    },
    telemetry::{Stage, StageTimes, TelemetryCounters, TelemetryEvent}, 
    types::StreamError, 
    utils::tracing_logger
};
use crate::stream_v3::{pipeline::{Monitor, PipelineMonitor}, segment_worker::SegmentInput};

pub fn run_compress_worker(
    rx: Receiver<SegmentInput>,
    tx: Sender<SegmentInput>,   // 👈 plain segment
    mut backend: Box<dyn CompressionBackend>,
    scheduler: Arc<Mutex<Scheduler>>,
    monitor: Monitor,
) {
    tracing_logger(Some(tracing::Level::DEBUG));
    debug!("[COMPRESSION WORKER] started");
    
    while let Ok(mut seg) = rx.recv() {
        if monitor.is_cancelled() {
            warn!("[COMPRESSION WORKER] cancelled, exiting loop");
            break;
        }

        let mut stage_times = StageTimes::default();
        let start = Instant::now();

        let target = {
            let mut sched = scheduler.lock().unwrap();
            sched.dispatch(seg.bytes.len())
        };

        // ✅ Final empty segment bypass
        if seg.flags.contains(SegmentFlags::FINAL_SEGMENT) && seg.bytes.is_empty() {
            stage_times.add(Stage::Compress, start.elapsed());
            let _ = tx.send(seg);

            monitor.report_telemetry(TelemetryEvent::StageSnapshot {
                stage_times: stage_times,
                counters: TelemetryCounters {
                    ..Default::default()
                },
            });

            let mut sched = scheduler.lock().unwrap();
            sched.complete(target);
            continue;
        }

        match backend.compress_chunk(&seg.bytes) {
            Ok(buf) => {
                seg.bytes = Bytes::from(buf);
                stage_times.add(Stage::Compress, start.elapsed());
                let len = seg.bytes.len() as u64;
                let _ = tx.send(seg);

                monitor.report_telemetry(TelemetryEvent::StageSnapshot {
                    stage_times: stage_times,
                    counters: TelemetryCounters {
                        bytes_compressed: len,
                        ..Default::default()
                    },
                });
            }
            Err(e) => {
                monitor.report_error(StreamError::Compression(e));
                break; // exit on error
            }
        }

        let mut sched = scheduler.lock().unwrap();
        sched.complete(target);
    }

    drop(tx); // ✅ ensures channel closure once worker exits

}

pub fn run_decompress_worker(
    rx: Receiver<DecryptedSegment>,
    tx: Sender<DecryptedSegment>,   // 👈 plain segment
    mut backend: Box<dyn CompressionBackend>,
    scheduler: Arc<Mutex<Scheduler>>,
    monitor: Monitor,
) {
    tracing_logger(Some(tracing::Level::DEBUG));
    debug!("[DECOMPRESSION WORKER] started");

    while let Ok(mut seg) = rx.recv() {
        if monitor.is_cancelled() {
            warn!("[DECOMPRESSION WORKER] cancelled, exiting loop");
            break;
        }

        let mut stage_times = StageTimes::default();
        let start = Instant::now();

        let target = {
            let mut sched = scheduler.lock().unwrap();
            sched.dispatch(seg.bytes.len())
        };

        if seg.header.flags().contains(SegmentFlags::FINAL_SEGMENT) && seg.bytes.is_empty() {
            stage_times.add(Stage::Decompress, start.elapsed());
            seg.stage_times = stage_times.clone();
            let _ = tx.send(seg);

            monitor.report_telemetry(TelemetryEvent::StageSnapshot {
                stage_times: stage_times,
                counters: TelemetryCounters {
                    ..Default::default()
                },
            });

            let mut sched = scheduler.lock().unwrap();
            sched.complete(target);
            continue;
        }

        match backend.decompress_chunk(&seg.bytes) {
            Ok(buf) => {
                seg.bytes = Bytes::from(buf);
                stage_times.add(Stage::Decompress, start.elapsed());
                seg.stage_times = stage_times.clone();
                let len = seg.bytes.len() as u64;
                let _ = tx.send(seg);

                monitor.report_telemetry(TelemetryEvent::StageSnapshot {
                    stage_times: stage_times,
                    counters: TelemetryCounters {
                        bytes_plaintext: len,
                        ..Default::default()
                    },
                });
            }
            Err(e) => {
                monitor.report_error(StreamError::Compression(e));
                break;
            }
        }

        let mut sched = scheduler.lock().unwrap();
        sched.complete(target);
    }
}
