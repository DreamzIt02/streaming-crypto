// ## 📦 `src/stream_v3/pipeline/pipeline.rs`

// ## Pure pipeline wiring (no crypto logic)

use std::io::Write;
use std::sync::atomic::{Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Instant};
use crossbeam::channel::{bounded};
use tracing::{debug, error, info, warn};

use crate::recovery::AsyncLogManager;
use crate::headers::HeaderV1;
use crate::segmenting::types::SegmentFlags;
use crate::stream_v2::{io, compression_worker::CodecInfo, pipeline::PipelineConfig, segment_worker::{EncryptContext, EncryptedSegment, DecryptContext, DecryptedSegment}};
use crate::stream_v3::{
    segment_worker::{DecryptSegmentWorker3, EncryptSegmentWorker3, SegmentInput},
        pipeline::{PipelineMonitor, Monitor, spawn_compress_workers_scoped, spawn_decompress_workers_scoped, spawn_decrypt_readers_scoped, spawn_encrypt_readers_scoped}
};
use crate::telemetry::{Stage, StageTimes, TelemetryCounters, TelemetryEvent, TelemetrySnapshot, TelemetryTimer};
use crate::types::StreamError;
use crate::utils::tracing_logger;

// ============================================================
// 🔓 Encrypt pipeline
// ============================================================
pub fn encrypt_pipeline<W>(
    input       : io::InputSource,
    mut writer  : W,
    crypto      : Arc<EncryptContext>,
    config      : &PipelineConfig,
    log_manager : Arc<AsyncLogManager>,
) -> Result<TelemetrySnapshot, StreamError>
where
    W: Write + Send,
{
    tracing_logger(Some(tracing::Level::DEBUG));
    debug!("[PIPELINE] Start encrypt pipeline");

    let mut counters = TelemetryCounters::default();
    let mut timer = TelemetryTimer::new();
    let mut last_segment_index = 0u32;

    // Header validation + emission
    let start = Instant::now();
    crypto.header.validate().map_err(StreamError::Header)?;
    timer.stage_times.add(Stage::Validate, start.elapsed());

    let start = Instant::now();
    io::write_header(&mut writer, &crypto.header)?;
    timer.stage_times.add(Stage::Write, start.elapsed());
    counters.bytes_overhead += HeaderV1::LEN as u64;

    // Channels (all plain segments now)
    let (comp_tx, comp_rx) = bounded::<SegmentInput>(config.profile.inflight_segments());
    let (seg_tx , seg_rx) = bounded::<SegmentInput>(config.profile.inflight_segments());
    let (out_tx, out_rx) = bounded::<EncryptedSegment>(config.profile.inflight_segments());

    // Local Monitor
    // let comp_txm      = comp_tx.clone(); 
    // let seg_txm       = seg_tx.clone(); 
    // let out_txm   = out_tx.clone(); 
    let comp_rxm    = comp_rx.clone(); 
    let seg_rxm     = seg_rx.clone(); 
    let out_rxm = out_rx.clone(); 

    let (monitor, monitor_rx) = Monitor::new(
        vec![
            // Box::new(comp_txm), 
            // Box::new(seg_txm), 
            // Box::new(out_txm), 
            // ❌ never put senders here — they cause the deadlock
            // Workers blocked on recv() are unblocked by dropping SENDERS, not receivers
        ],
        vec![
            Box::new(comp_rxm),
            Box::new(seg_rxm),
            Box::new(out_rxm),
        ],
    );
    
    // Telemetry state
    let writer_stage_times = Arc::new(Mutex::new(StageTimes::default()));

    // Store first fatal error for propagation
    let fatal_error: Arc<Mutex<Option<StreamError>>> = Arc::new(Mutex::new(None));

    // ADD before scope:
    let global_timer_m    = Arc::new(Mutex::new(StageTimes::default()));
    let global_counters_m = Arc::new(Mutex::new(TelemetryCounters::default()));

    let _ = crossbeam::thread::scope(|scope| {
        // ===============================================================
        // Monitor thread
        // ===============================================================
        let monitor_m = monitor.clone();
        let fatal_error_m = fatal_error.clone(); // Arc<Mutex<Option<StreamError>>>
        // Clone for monitor thread:
        let global_timer_t    = global_timer_m.clone();
        let global_counters_t = global_counters_m.clone();

        scope.spawn(move |_| {
            loop {
                match monitor_rx.recv() {
                    Ok(Ok(TelemetryEvent::StageSnapshot { stage_times, counters })) => {
                        debug!("[MONITOR] telemetry snapshot received");
                        global_timer_t.lock().unwrap().merge(&stage_times);
                        global_counters_t.lock().unwrap().merge(&counters);
                    }
                    Ok(Ok(TelemetryEvent::PipelineFinished { final_stage_times, final_counters })) => {
                        info!("[MONITOR] pipeline finished telemetry");
                        global_timer_t.lock().unwrap().merge(&final_stage_times);
                        global_counters_t.lock().unwrap().merge(&final_counters);
                    }
                    Ok(Err(err)) => {
                        error!("[MONITOR] fatal error: {}", err);
                        *fatal_error_m.lock().unwrap() = Some(err);

                        // Cancel pipeline immediately
                        monitor_m.cancelled.store(true, Ordering::Relaxed);
                        // Kill all senders
                        monitor_m.senders.lock().unwrap().clear();
                        monitor_m.receivers.lock().unwrap().clear();
                        break;
                    }
                    Err(_) => {
                        // Channel closed normally -> pipeline completed successfully
                        warn!("[MONITOR] channel closed, exiting");
                        break;
                    }
                }
            }
        });

        // ===============================================================
        // Reader workers
        // ===============================================================
        let profile_r = config.profile.clone();
        let chunk_size = crypto.base.segment_size;
        let tx_r = comp_tx.clone();
        let monitor_r = monitor.clone();

        spawn_encrypt_readers_scoped(
            scope,
            input,
            chunk_size,
            profile_r,
            tx_r,
            monitor_r,
        );
        // comp_tx moved in — no clone, no manual drop needed
        drop(comp_tx);

        // ===============================================================
        // Compression workers
        // ===============================================================
        let mut codec_info = CodecInfo::from_header(&crypto.header, None);
        codec_info.gpu = config.profile.gpu();

        let profile_c = config.profile.clone();
        // let rx_c = comp_rx.clone();
        let tx_c = seg_tx.clone();
        let monitor_c = monitor.clone();

        spawn_compress_workers_scoped(
            scope,
            profile_c,
            codec_info,
            comp_rx,
            tx_c,
            monitor_c,
        );
        // comp_rx and seg_tx_clean moved in — no clone, no manual drop
        // drop(comp_rx);
        drop(seg_tx);
        
        // ===============================================================
        // Crypto workers
        // ===============================================================
        for _ in 0..config.profile.cpu_workers() {
            if monitor.is_cancelled() {
                warn!("[SPAWN CRYPTO] cancelled, exiting early");
                break;
            }
            let crypto_w = crypto.clone();
            let log_w = log_manager.clone(); 
            let rx = seg_rx.clone();    // ← clone for each worker
            let tx = out_tx.clone();  // ← clone for each worker
            let monitor_w = monitor.clone();

            scope.spawn(move |_| {
                let worker = EncryptSegmentWorker3::new(
                    crypto_w, 
                    log_w, 
                    monitor_w
                );
                worker.run(rx, tx);
            });
        }

        drop(seg_rx);   // ← original dropped after all clones made
        drop(out_tx);   // ← original dropped after all clones made

        // ===============================================================
        // Ordered writer
        // ===============================================================
        let mut ordered_writer = io::OrderedEncryptedWriter::new(&mut writer);

        // out_rx is already the raw original — used directly, no clone needed
        for seg in out_rx.iter() {
            if monitor.is_cancelled() {
                warn!("[WRITER] cancelled, exiting early");
                break;
            }

            let start = Instant::now();
            let is_final = seg.header.flags().contains(SegmentFlags::FINAL_SEGMENT);
            let idx = seg.header.segment_index();

            if let Err(e) = ordered_writer.push(seg) {
                monitor.report_error(e);
                break;
            }

            writer_stage_times.lock().unwrap().add(Stage::Write, start.elapsed());

            if is_final {
                debug!("[WRITER] final segment {} written, exiting loop", idx);
                last_segment_index = idx;
            }
        }

        if let Err(e) = ordered_writer.finish() {
            monitor.report_error(e);
        } 
        else {
            monitor.report_telemetry(TelemetryEvent::PipelineFinished {
                final_stage_times: writer_stage_times.lock().unwrap().clone(),
                final_counters: TelemetryCounters::default(), // ← workers already reported via StageSnapshot
            });
        }

        monitor.finish(); // clear senders/receivers

        Ok::<(), StreamError>(())
    })
    .map_err(|e| {
        // Try to downcast the panic payload into something printable
        if let Some(s) = e.downcast_ref::<&str>() {
            StreamError::PipelineError(format!("Thread panicked: {}", s))
        } else if let Some(s) = e.downcast_ref::<String>() {
            StreamError::PipelineError(format!("Thread panicked: {}", s))
        } else {
            StreamError::PipelineError("Thread panicked with unknown payload".to_string())
        }
    })?;

    // ✅ Check fatal error BEFORE returning Ok
    if let Some(err) = fatal_error.lock().unwrap().take() {
        return Err(err);
    }

    // Final telemetry aggregation
    timer.finish();

    // Replace entire manual aggregation block at the bottom with:
    timer.stage_times.merge(&global_timer_m.lock().unwrap());
    counters.merge(&global_counters_m.lock().unwrap());

    Ok(TelemetrySnapshot::from(&counters, &timer, Some(last_segment_index)))
}


// ============================================================
// 🔓 Decrypt pipeline (v3)
// ============================================================
pub fn decrypt_pipeline<W>(
    input       : io::InputSource,
    mut writer  : W,
    crypto      : Arc<DecryptContext>,
    config      : &PipelineConfig,
    log_manager : Arc<AsyncLogManager>,
) -> Result<TelemetrySnapshot, StreamError>
where
    W: Write + Send,
{
    tracing_logger(Some(tracing::Level::DEBUG));
    debug!("[PIPELINE] Start decrypt pipeline");

    let mut counters = TelemetryCounters::default();
    let mut timer = TelemetryTimer::new();
    let mut last_segment_index = 0u32;

    // Header validation
    let start = Instant::now();
    crypto.header.validate().map_err(StreamError::Header)?;
    timer.stage_times.add(Stage::Validate, start.elapsed());
    counters.bytes_overhead += HeaderV1::LEN as u64;

    // Channels
    let (seg_tx, seg_rx) = bounded::<SegmentInput>(config.profile.inflight_segments());
    let (crypto_tx, crypto_rx) = bounded::<DecryptedSegment>(config.profile.inflight_segments());
    let (decomp_tx, decomp_rx) = bounded::<DecryptedSegment>(config.profile.inflight_segments());

    // Local Monitor
    // let seg_txm      = seg_tx.clone(); 
    // let seg_txm       = crypto_tx.clone(); 
    // let decomp_txm   = decomp_tx.clone(); 
    let seg_rxm    = seg_rx.clone(); 
    let crypto_rxm     = crypto_rx.clone(); 
    let decomp_rxm = decomp_rx.clone(); 

    let (monitor, monitor_rx) = Monitor::new(
        vec![
            // Box::new(seg_txm), 
            // Box::new(seg_txm), 
            // Box::new(decomp_txm), 
            // ❌ never put senders here — they cause the deadlock
            // Workers blocked on recv() are unblocked by dropping SENDERS, not receivers
        ],
        vec![
            Box::new(seg_rxm),
            Box::new(crypto_rxm),
            Box::new(decomp_rxm),
        ],
    );

    // Telemetry state
    let writer_stage_times = Arc::new(Mutex::new(StageTimes::default()));

    // Store first fatal error for propagation
    let fatal_error: Arc<Mutex<Option<StreamError>>> = Arc::new(Mutex::new(None));

    // ADD before scope:
    let global_timer_m    = Arc::new(Mutex::new(StageTimes::default()));
    let global_counters_m = Arc::new(Mutex::new(TelemetryCounters::default()));

    let _ = crossbeam::thread::scope(|scope| {
        // ===============================================================
        // Monitor thread
        // ===============================================================
        let monitor_m = monitor.clone();
        let fatal_error_m = fatal_error.clone();
        // Clone for monitor thread:
        let global_timer_t    = global_timer_m.clone();
        let global_counters_t = global_counters_m.clone();

        scope.spawn(move |_| {
            loop {
                match monitor_rx.recv() {
                    Ok(Ok(TelemetryEvent::StageSnapshot { stage_times, counters })) => {
                        debug!("[MONITOR] telemetry snapshot received");
                        global_timer_t.lock().unwrap().merge(&stage_times);
                        global_counters_t.lock().unwrap().merge(&counters);
                    }
                    Ok(Ok(TelemetryEvent::PipelineFinished { final_stage_times, final_counters })) => {
                        info!("[MONITOR] pipeline finished telemetry");
                        global_timer_t.lock().unwrap().merge(&final_stage_times);
                        global_counters_t.lock().unwrap().merge(&final_counters);
                    }
                    Ok(Err(err)) => {
                        error!("[MONITOR] fatal error: {}", err);
                        *fatal_error_m.lock().unwrap() = Some(err);

                        // Cancel pipeline immediately
                        monitor_m.cancelled.store(true, Ordering::Relaxed);
                        // Kill all senders
                        monitor_m.senders.lock().unwrap().clear();
                        monitor_m.receivers.lock().unwrap().clear();
                        break;
                    }
                    Err(_) => {
                        warn!("[MONITOR] channel closed, exiting");
                        break;
                    }
                }
            }
        });

        // ===============================================================
        // Reader workers (ciphertext → segments)
        // ===============================================================
        let profile_r = config.profile.clone();
        let chunk_size = crypto.base.segment_size;
        let tx_r = seg_tx.clone();
        let monitor_r = monitor.clone();

        spawn_decrypt_readers_scoped(
            scope,
            input,
            chunk_size,
            profile_r,
            tx_r,
            monitor_r,
        );
        // seg_tx moved in — no clone, no manual drop needed
        drop(seg_tx);

        // ===============================================================
        // Crypto workers (decrypt)
        // ===============================================================
        for _ in 0..config.profile.cpu_workers() {
            let crypto_w = crypto.clone();
            let log_w = log_manager.clone();
            let rx = seg_rx.clone();        // ← clone for each worker
            let tx = crypto_tx.clone();   // ← clone for each worker
            let monitor_w = monitor.clone();

            scope.spawn(move |_| {
                let worker = DecryptSegmentWorker3::new(crypto_w, log_w, monitor_w);
                worker.run(rx, tx);
            });
        }

        drop(seg_rx);       // ← original dropped after all clones made
        drop(crypto_tx);    // ← original dropped after all clones made

        // ===============================================================
        // Decompression workers
        // ===============================================================
        let mut codec_info = CodecInfo::from_header(&crypto.header, None);
        codec_info.gpu = config.profile.gpu();

        let profile_d = config.profile.clone();
        let tx_d = decomp_tx.clone();
        let monitor_d = monitor.clone();

        spawn_decompress_workers_scoped(
            scope,
            profile_d,
            codec_info,
            crypto_rx,
            tx_d,
            monitor_d,
        );
        // decomp_tx moved in — no clone, no manual drop
        drop(decomp_tx);

        // ===============================================================
        // Ordered plaintext writer
        // ===============================================================
        let mut ordered_writer = io::OrderedPlaintextWriter::new(&mut writer);

        for seg in decomp_rx.iter() {
            let start = Instant::now();
            let is_final = seg.header.flags().contains(SegmentFlags::FINAL_SEGMENT);
            let idx = seg.header.segment_index();

            if let Err(e) = ordered_writer.push(&seg) {
                monitor.report_error(e);
                break;
            }

            if is_final {
                debug!("[WRITER] final segment {} written, exiting loop", idx);
                last_segment_index = idx;
            }

            // record write time
            writer_stage_times.lock().unwrap().add(Stage::Write, start.elapsed());
        }

        if let Err(e) = ordered_writer.finish() {
            monitor.report_error(e);
        }
        else {
            monitor.report_telemetry(TelemetryEvent::PipelineFinished {
                final_stage_times: writer_stage_times.lock().unwrap().clone(),
                final_counters: TelemetryCounters::default(), // ← workers already reported via StageSnapshot
            });
        }

        monitor.finish();

        Ok::<(), StreamError>(())
    })
    .map_err(|e| {
        if let Some(s) = e.downcast_ref::<&str>() {
            StreamError::PipelineError(format!("Thread panicked: {}", s))
        } else if let Some(s) = e.downcast_ref::<String>() {
            StreamError::PipelineError(format!("Thread panicked: {}", s))
        } else {
            StreamError::PipelineError("Thread panicked with unknown payload".into())
        }
    })?;

    // Check if a fatal error occurred
    if let Some(err) = fatal_error.lock().unwrap().take() {
        return Err(err);
    }

    // Telemetry aggregation
    timer.finish();
   
    // Replace entire manual aggregation block at the bottom with:
    timer.stage_times.merge(&global_timer_m.lock().unwrap());
    counters.merge(&global_counters_m.lock().unwrap());

    Ok(TelemetrySnapshot::from(&counters, &timer, Some(last_segment_index)))
}
