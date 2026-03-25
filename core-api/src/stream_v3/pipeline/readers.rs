// # 🧱 1. Entry Dispatcher

use std::io::{self, Read, Seek};

use std::fs::File;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use std::time::Instant;

use bytes::Bytes;
use crossbeam::channel::Sender;
use crossbeam::thread::Scope;
use tracing::{debug, info, warn};

use crate::headers::HeaderV1;
use crate::io::{InputSource, read_header};
use crate::parallelism::HybridParallelismProfile;
use crate::stream_v3::{pipeline::types::{Monitor, PipelineMonitor}, segment_worker::{types::{SegmentInput}}};
use crate::segmenting::{SegmentHeader, decode_segment_header};
use crate::segmenting::types::SegmentFlags;
use crate::telemetry::{Stage, StageTimes, TelemetryCounters, TelemetryEvent};
use crate::types::StreamError;
use crate::utils::tracing_logger;

pub fn decrypt_read_header<'a>(
    input: InputSource<'a>,
) -> Result<(HeaderV1, InputSource<'a>), StreamError> {
    match input {
        InputSource::Memory(data) => {
            let mut cursor = std::io::Cursor::new(data);
            let header = read_header(&mut cursor)?;
            let pos = cursor.position() as usize;
            Ok((header, InputSource::Memory(&data[pos..])))
        }
        InputSource::File(path) => {
            let mut file = std::fs::File::open(&path)?;
            let header = read_header(&mut file)?;
            // For File, we don’t slice, but we’ll pass the path and a start_offset to readers
            Ok((header, InputSource::File(path)))
        }
        InputSource::Reader(mut reader) => {
            let header = read_header(&mut reader)?;
            Ok((header, InputSource::Reader(reader)))
        }
    }
}

pub fn spawn_encrypt_readers_scoped<'scope>(
    scope: &Scope<'scope>,
    input: InputSource<'scope>,
    chunk_size: usize,
    profile: HybridParallelismProfile,
    tx: Sender<SegmentInput>,
    monitor: Monitor,
) {
    tracing_logger(Some(tracing::Level::DEBUG));

    debug!("[SPAWN READERS] spawning CPU={}, GPU={} workers", profile.cpu_workers(), profile.gpu_workers());
    match input {
        InputSource::Memory(data) => {
            debug!("[SPAWN READERS] Input Source Detected (Memory)");
            spawn_memory_reader_enc(scope, data, chunk_size, profile, tx, monitor);
        }

        InputSource::File(path) => {
            debug!("[SPAWN READERS] Input Source Detected (File)");
            // Try to open file
            let file = match File::open(&path) {
                Ok(f) => Arc::new(f),
                Err(e) => {
                    monitor.report_error(StreamError::IoError(e.kind(), e.to_string()));
                    return;
                }
            };
            spawn_pread_reader_enc(scope, (file, 0), chunk_size, profile, tx, monitor);
        }

        InputSource::Reader(reader) => {
            debug!("[SPAWN READERS] Input Source Detected (Reader)");
            spawn_stream_reader_enc(scope, reader, chunk_size, profile, tx, monitor);
        }
    }
}

pub fn spawn_decrypt_readers_scoped<'scope>(
    scope: &Scope<'scope>,
    input: InputSource<'scope>,
    chunk_size: usize,
    profile: HybridParallelismProfile,
    tx: Sender<SegmentInput>,
    monitor: Monitor,
) {
    tracing_logger(Some(tracing::Level::DEBUG));
    debug!("[SPAWN DECRYPT READERS] spawning CPU={}, GPU={} workers", profile.cpu_workers(), profile.gpu_workers());

    match input {
        InputSource::Memory(data) => {
            info!("[SPAWN READERS] Input Source Detected (Memory)");
            spawn_memory_reader_dec(scope, data, chunk_size, profile, tx, monitor);
        }

        InputSource::File(path) => {
            info!("[SPAWN READERS] Input Source Detected (File)");
            // Try to open file
            let file: Arc<File> = match File::open(&path) {
                Ok(f) => Arc::new(f),
                Err(e) => {
                    monitor.report_error(StreamError::IoError(e.kind(), e.to_string()));
                    return;
                }
            };

            spawn_pread_reader_dec(scope, (file, HeaderV1::LEN), chunk_size, profile, tx, monitor);
        }

        InputSource::Reader(reader) => {
            info!("[SPAWN READERS] Input Source Detected (Reader)");
            spawn_stream_reader_dec(scope, reader, chunk_size, profile, tx, monitor);
        }
    }
}

fn spawn_reader_common<'scope, F>(
    scope       : &Scope<'scope>,
    workers     : usize,
    tx          : Sender<SegmentInput>,
    monitor     : Monitor,
    make_reader : impl Fn(usize) -> F,   // 👈 factory for per-worker closure
    on_all_done : impl FnOnce() + Send + 'scope,  // ← called when last worker exits
)
where
    F: FnMut() -> Option<(u32, Bytes)> + Send + 'scope,
{
    let remaining = Arc::new(AtomicUsize::new(workers));
    let on_all_done = Arc::new(Mutex::new(Some(on_all_done))); // ← wrap here

    for worker_id in 0..workers {
        if monitor.is_cancelled() {
            warn!("[SPAWN READERS] cancelled, exiting loop");
            // Adjust remaining down for unspawned workers
            remaining.fetch_sub(workers - worker_id, Ordering::AcqRel);
            break;
        }

        let tx        = tx.clone();
        let monitor   = monitor.clone();
        let remaining = remaining.clone();
        let on_all_done = on_all_done.clone(); // ← Arc clone, not FnOnce clone
        
        // build a fresh closure for this worker
        let mut read_next = make_reader(worker_id);

        scope.spawn(move |_| {
            let thread = std::thread::current();
            info!(
                "[SPAWN READERS] started: worker_id={}, thread_id={:?}, name={:?}",
                worker_id, thread.id(), thread.name()
            );

            while let Some((index, bytes)) = read_next() {
                if monitor.is_cancelled() {
                    warn!("[SPAWN READERS] cancelled, exiting loop");
                    break;
                }

                let start_time = Instant::now();
                let len = bytes.len();
                debug!("[SPAWN READERS] dispatch data segment, {}", index);
                let _ = tx.send(SegmentInput {
                    index   : index,
                    bytes   : bytes,
                    flags   : SegmentFlags::empty(),
                    header  : SegmentHeader::default(),
                });
                let duration = start_time.elapsed();

                let mut stage_times = StageTimes::default();
                stage_times.times.insert(Stage::Read, duration);
                let counters = TelemetryCounters {
                    bytes_plaintext: len as u64,
                    ..Default::default()
                };
                monitor.report_telemetry(TelemetryEvent::StageSnapshot { stage_times, counters });

                if monitor.is_cancelled() { break; }
            }
            // ✅ ensures channel closure once worker exits
            drop(tx); // drop before decrement — channel must close before on_all_done fires
           
            // ── Last worker out sends the final segment ──────────────────
            if remaining.fetch_sub(1, Ordering::AcqRel) == 1 {
                // Last worker: take the FnOnce out of the Option and call it
                if let Some(f) = on_all_done.lock().unwrap().take() {
                    f(); // ✅ called exactly once
                }
            }
            warn!(
                "[SPAWN READERS] exiting: worker_id={}, thread_id={:?}, name={:?}",
                worker_id, thread.id(), thread.name()
            );
        });
    }

    drop(tx); // ✅ drop the original so only worker clones keep it alive

}

fn send_final_segment(
    tx      : Sender<SegmentInput>,
    monitor : Monitor,
    final_index: u32,
) {
    if final_index == 0 {
        monitor.report_error(StreamError::Validation("Empty input: no segments dispatched".into()));
        drop(tx);
        return;
    }

    let _ = tx.send(SegmentInput {
        index  : final_index,
        bytes  : Bytes::new(),
        flags  : SegmentFlags::FINAL_SEGMENT,
        header : SegmentHeader::default(),
    });

    drop(tx); // ✅ drop the original so only worker clones keep it alive
}

fn read_segment_into<R: Read>(
    reader  : &mut R,
    tx      : &Sender<SegmentInput>,
    monitor : &Monitor,
) -> Result<bool, StreamError> {
    let mut hdr_buf = [0u8; SegmentHeader::LEN];
    if let Err(_) = reader.read_exact(&mut hdr_buf) {
        return Ok(false); // EOF
    }

    let header = decode_segment_header(&hdr_buf).map_err(StreamError::Segment)?;
    let mut wire = vec![0u8; header.wire_len() as usize];
    if header.wire_len() > 0 {
        reader.read_exact(&mut wire)?;
    }

    let _ = tx.send(SegmentInput {
        index   : header.segment_index(),
        bytes   : Bytes::from(wire),
        flags   : header.flags(),
        header  : header,
    });

    // Telemetry snapshot
    let mut stage_times = StageTimes::default();
    stage_times.times.insert(Stage::Read, Instant::now().elapsed());
    let counters = TelemetryCounters {
        bytes_ciphertext: header.wire_len() as u64,
        ..Default::default()
    };
    monitor.report_telemetry(TelemetryEvent::StageSnapshot { stage_times, counters });

    Ok(true)
}


// # 🚀 2. Memory Reader (🔥 fastest path)

fn spawn_memory_reader_enc<'scope>(
    scope   : &Scope<'scope>,
    data    : &'scope [u8],
    chunk_size: usize,
    profile : HybridParallelismProfile,
    tx      : Sender<SegmentInput>,
    monitor : Monitor,
) {
    let len = data.len();
    let total_segments = (len + chunk_size - 1) / chunk_size;
    let workers = (profile.cpu_workers() / 2).max(1);
    let seg_index   = Arc::new(AtomicUsize::new(0));
    // Instead track a separate "last dispatched" atomic:
    let last_dispatched = Arc::new(AtomicUsize::new(0));

    let last_dispatched_clone = last_dispatched.clone();
    let tx_final        = tx.clone();
    let monitor_final   = monitor.clone();

    spawn_reader_common(scope, workers, tx, monitor, 
        move |_| {
            // per-worker read closure (unchanged)
            let seg_index = seg_index.clone();
            let last_dispatched = last_dispatched_clone.clone();
            move || {
                let idx = seg_index.fetch_add(1, Ordering::Relaxed);
                if idx >= total_segments {
                    return None;
                }
                last_dispatched.store(idx + 1, Ordering::Release); // ← track actual count

                let start = idx * chunk_size;
                let end = ((idx + 1) * chunk_size).min(len);
                let slice = &data[start..end];
                Some((idx as u32, Bytes::copy_from_slice(slice)))
            }
        },
        move || {
            // ← runs in the last exiting worker's thread, after its tx is dropped
            let final_index = last_dispatched.load(Ordering::Acquire) as u32;
            debug!("[MEMORY READER] Last worker done. final_index={}", final_index);
            
            // ✅ Send final empty segment to mark EOF
            send_final_segment(tx_final, monitor_final, final_index);
        }
    );

}

fn spawn_memory_reader_dec<'scope>(
    scope   : &Scope<'scope>,
    data    : &'scope [u8],
    _chunk_size: usize, // unused for decrypt
    profile : HybridParallelismProfile,
    tx      : Sender<SegmentInput>,
    monitor : Monitor,
) {
    let workers = (profile.cpu_workers() / 2).max(1).min(1); // keep symmetry

    for worker_id in 0..workers {
        if monitor.is_cancelled() {
            warn!("[MEMORY READER] cancelled, exiting loop");
            break;
        }
        let mut cursor = std::io::Cursor::new(data);
        let tx_w = tx.clone();
        let monitor = monitor.clone();

        scope.spawn(move |_| {
            info!("[MEMORY READER] Memory reader worker={}", worker_id);
            loop {
                if monitor.is_cancelled() {
                    warn!("[MEMORY READER] cancelled, exiting loop");
                    break;
                }

                match read_segment_into(&mut cursor, &tx_w, &monitor) {
                    Ok(true) => continue,
                    Ok(false) => break,
                    Err(e) => { monitor.report_error(e); break; }
                }
            }
            drop(tx_w); // ✅ drop the original so only worker clones keep it alive
        });
    }
    drop(tx); // ✅ ensures channel closure once worker exits
}

// # 🚀 3. File Reader (🔥 pread parallel)

/// Cross-platform trait for `read_at`
pub trait FileReadAt {
    fn read_at(&self, buf: &mut [u8], offset: u64) -> io::Result<usize>;
}

#[cfg(unix)]
impl FileReadAt for File {
    fn read_at(&self, buf: &mut [u8], offset: u64) -> io::Result<usize> {
        use std::os::unix::fs::FileExt;
        FileExt::read_at(self, buf, offset)
    }
}

#[cfg(windows)]
impl FileReadAt for File {
    fn read_at(&self, buf: &mut [u8], offset: u64) -> io::Result<usize> {
        use std::io::{SeekFrom};
        // emulate pread by cloning handle and seeking
        let mut f = self.try_clone()?;
        f.seek(SeekFrom::Start(offset))?;
        f.read(buf)
    }
}

fn spawn_pread_reader_enc<'scope>(
    scope      : &Scope<'scope>,
    file_input : (Arc<File>, usize),
    chunk_size : usize,
    profile    : HybridParallelismProfile,
    tx         : Sender<SegmentInput>,
    monitor    : Monitor,
) {
    let (file, start_offset) = file_input;
    let file_size   = file.metadata().map(|m| m.len()).unwrap_or(0);
    let next_offset = Arc::new(AtomicU64::new(start_offset as u64));
    let workers     = (profile.cpu_workers() / 2).max(1);
    let seg_index   = Arc::new(AtomicUsize::new(0));

    let seg_index_clone = seg_index.clone();
    let tx_final        = tx.clone();
    let monitor_final   = monitor.clone();

    spawn_reader_common(scope, workers, tx, monitor.clone(), 
        move |_| {
            let file = file.clone();
            let next_offset = next_offset.clone();
            let seg_index = seg_index_clone.clone();
            let monitor_r = monitor.clone();
            
            move || {
                let offset = next_offset.fetch_add(chunk_size as u64, Ordering::Relaxed);
                if offset >= file_size {
                    return None; // EOF
                }

                let mut buf = vec![0u8; chunk_size];
                match file.read_at(&mut buf, offset) {
                    Ok(0) => {
                        // EOF
                        None
                    }
                    Ok(n) => {
                        // normal segment, n > 0
                        let idx = seg_index.fetch_add(1, Ordering::Relaxed);
                        Some((idx as u32, Bytes::copy_from_slice(&buf[..n])))
                    }
                    Err(e) => {
                        // ❌ Fatal error: propagate to monitor
                        monitor_r.report_error(StreamError::IoError(e.kind(), e.to_string()));
                        None
                    }
                }
            }
        },
        move || {
            // ✅ runs in last exiting worker's thread, after its tx is dropped
            let final_index = seg_index.load(Ordering::Acquire) as u32;
            info!("[FILE READER] Last worker done. final_index={}", final_index);
            
            // ✅ Send final empty segment to mark EOF
            send_final_segment(tx_final, monitor_final, final_index);
        },
    );
}

fn spawn_pread_reader_dec<'scope>(
    scope   : &Scope<'scope>,
    file_input: (Arc<File>, usize), // 👈 include offset
    _chunk_size: usize,
    profile : HybridParallelismProfile,
    tx      : Sender<SegmentInput>,
    monitor : Monitor,
) {
    let (file, offset) = file_input;
    let workers = (profile.cpu_workers() / 2).max(1).min(1); // keep symmetry

    for worker_id in 0..workers {
        if monitor.is_cancelled() { break; }
        let tx_w = tx.clone();
        let monitor = monitor.clone();
        let file = file.clone();

        scope.spawn(move |_| {
            info!("[SPAWN READERS] File decrypt worker={}", worker_id);
            let mut reader = file.as_ref();
            // Advance header offset before reading segments
            let _ = reader.seek(std::io::SeekFrom::Start(offset as u64));

            loop {
                if monitor.is_cancelled() {
                    warn!("[SPAWN READERS] cancelled, exiting loop");
                    break;
                }

                match read_segment_into(&mut reader, &tx_w, &monitor) {
                    Ok(true) => continue,
                    Ok(false) => break,
                    Err(e) => { monitor.report_error(e); break; }
                }
            }
            drop(tx_w); // ✅ drop the original so only worker clones keep it alive
        });
    }
    drop(tx); // ✅ ensures channel closure once worker exits
}

// # 🚀 4. Stream Reader (fallback, correct)

fn spawn_stream_reader_enc<'scope>(
    scope      : &Scope<'scope>,
    reader     : Box<dyn Read + Send>,
    chunk_size : usize,
    profile    : HybridParallelismProfile,
    tx         : Sender<SegmentInput>,
    monitor    : Monitor,
) {
    let workers   = (profile.cpu_workers() / 2).max(1).min(1);
    let reader    = Arc::new(Mutex::new(reader));
    let seg_index = Arc::new(AtomicUsize::new(0));

    let seg_index_clone = seg_index.clone();
    let tx_final        = tx.clone();
    let monitor_final   = monitor.clone();

    spawn_reader_common(scope, workers, tx, monitor.clone(), 
        move |_| {
            let reader = reader.clone();
            let seg_index = seg_index_clone.clone();
            let monitor_r = monitor.clone();
            
            move || {
                let mut buf = vec![0u8; chunk_size];
                let mut guard = reader.lock().unwrap();
                match guard.read(&mut buf) {
                    Ok(0) => {
                        // EOF
                        None
                    }
                    Ok(n) => {
                        // normal segment, n > 0
                        let idx = seg_index.fetch_add(1, Ordering::Relaxed);
                        Some((idx as u32, Bytes::copy_from_slice(&buf[..n])))
                    }
                    Err(e) => {
                        // ❌ fatal error: report to monitor and stop
                        monitor_r.report_error(StreamError::IoError(e.kind(), e.to_string()));
                        None
                    }
                }
            }
        },
        move || {
            // ✅ runs in last exiting worker's thread, after its tx is dropped
            let final_index = seg_index.load(Ordering::Acquire) as u32;
            info!("[STREAM READER] Last worker done. final_index={}", final_index);
            
            // ✅ Send final empty segment to mark EOF
            send_final_segment(tx_final, monitor_final, final_index);
        },
    );
}

fn spawn_stream_reader_dec<'scope>(
    scope   : &Scope<'scope>,
    reader  : Box<dyn Read + Send>,
    _chunk_size: usize, // unused for decrypt
    profile : HybridParallelismProfile,
    tx      : Sender<SegmentInput>,
    monitor : Monitor,
) {
    let mut reader_opt = Some(reader);
    let workers = (profile.cpu_workers() / 2).max(1).min(1); // keep symmetry

    for worker_id in 0..workers {
        if monitor.is_cancelled() {
            warn!("[SPAWN READERS] cancelled, exiting loop");
            break;
        }
        let tx_w: Sender<SegmentInput> = tx.clone();
        let monitor = monitor.clone();
        let mut reader = reader_opt.take().expect("reader already moved");

        scope.spawn(move |_| {
            info!("[SPAWN READERS] Stream reader worker={}", worker_id);
            loop {
                if monitor.is_cancelled() {
                    warn!("[SPAWN READERS] cancelled, exiting loop");
                    break;
                }

                match read_segment_into(&mut reader, &tx_w, &monitor) {
                    Ok(true) => continue,
                    Ok(false) => break,
                    Err(e) => { monitor.report_error(e); break; }
                }
            }
            drop(tx_w); // ✅ drop the original so only worker clones keep it alive
        });
    }
    drop(tx); // ✅ ensures channel closure once worker exits
}

// # 🧠 What we just achieved

// | Path   | Copies before | Copies now         |
// | ------ | ------------- | ------------------ |
// | Memory | ❌ memcpy      | ✅ zero-copy       |
// | File   | ❌ 2 copies    | ✅ 1 (kernel only) |
// | Stream | ❌ extra copy  | ✅ minimal         |
