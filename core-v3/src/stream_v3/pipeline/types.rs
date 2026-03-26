// ## 📦 `src/stream_v3/pipeline/types.rs`

use std::{any::Any, sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}}};
use crossbeam::channel::{Receiver, Sender};

use core_api::{telemetry::TelemetryEvent, types::StreamError};

pub trait PipelineMonitor {
    fn report_error(&self, err: StreamError);
    fn report_telemetry(&self, t: TelemetryEvent);
    fn is_cancelled(&self) -> bool;
}

#[derive(Debug)]
pub struct Monitor {
    pub monitor_tx  : Sender<Result<TelemetryEvent, StreamError>>,
    pub cancelled   : Arc<AtomicBool>,
    pub cancel_tx   : Sender<()>,       // ← broadcast cancel signal
    pub cancel_rx   : Receiver<()>,     // ← every worker selects on this

    // Only populated on the pipeline-owned instance, empty on worker clones
    pub senders     : Arc<Mutex<Vec<Box<dyn Any + Send>>>>,
    // Only populated on the pipeline-owned instance, empty on worker clones
    pub receivers   : Arc<Mutex<Vec<Box<dyn Any + Send>>>>,
}

impl Clone for Monitor {
    fn clone(&self) -> Self {
        Self {
            monitor_tx  : self.monitor_tx.clone(),
            cancelled   : self.cancelled.clone(),
            cancel_tx   : self.cancel_tx.clone(),
            cancel_rx   : self.cancel_rx.clone(),
            senders     : Arc::new(Mutex::new(vec![])), // ← empty on clone
            receivers   : Arc::new(Mutex::new(vec![])), // ← empty on clone
        }
    }
}

impl PipelineMonitor for Monitor {
    #[inline]
    fn report_error(&self, err: StreamError) {
        // First error wins — if already cancelled, discard silently
        if self.cancelled.load(Ordering::Relaxed) {
            return;
        }

        match self.monitor_tx.send(Err(err)) {
            Ok(()) => eprintln!("[MONITOR] Error sent"),
            Err(e) => {
                // panic!("Fatal error could not be reported: {:?}", e.0);
                // Channel closed but not yet cancelled — log and discard, never panic
                eprintln!("[MONITOR] ❌ Lost error (channel closed): {:?}", e.0);
            }
        }
        self.cancelled.store(true, Ordering::Relaxed);
        
        let _ = self.cancel_tx.try_send(()); // ← unblocks all select! immediately
        self.senders.lock().unwrap().clear();
        self.receivers.lock().unwrap().clear();
    }

    #[inline]
    fn report_telemetry(&self, t: TelemetryEvent) {
        let _ = self.monitor_tx.send(Ok(t));
    }

    #[inline]
    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }
}

impl Monitor {
    pub fn new(
        senders: Vec<Box<dyn Any + Send>>,
        receivers: Vec<Box<dyn Any + Send>>,
    ) -> (Self, Receiver<Result<TelemetryEvent, StreamError>>) {
        let (monitor_tx, monitor_rx) = crossbeam::channel::unbounded();
        let cancelled = Arc::new(AtomicBool::new(false));
        let (cancel_tx, cancel_rx) = crossbeam::channel::bounded(1);
        (
            Self {
                cancelled,
                monitor_tx,
                cancel_tx,
                cancel_rx,
                senders     : Arc::new(Mutex::new(senders)),
                receivers   : Arc::new(Mutex::new(receivers)),
            },
            monitor_rx,
        )
    }

    pub fn cancel_rx(&self) -> &Receiver<()> {
        &self.cancel_rx  // ← workers call this to get their select handle
    }

    #[inline]
    pub fn finish(self) {
        self.senders.lock().unwrap().clear();
        self.receivers.lock().unwrap().clear();
    }
}
