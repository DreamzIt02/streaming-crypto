// ## 3. `src/parallelism/scheduler.rs`

#[derive(Debug, Clone, PartialEq)]
pub enum WorkerTarget {
    Cpu(usize), // index of CPU worker
    Gpu(usize), // index of GPU device
}

/// Decide where to dispatch a segment based on size and load.
pub fn dispatch_segment(
    segment_size: usize,
    cpu_workers: usize,
    gpu_workers: usize,
    gpu_threshold: usize, // e.g. 8 MB
    cpu_load: &[usize],   // queue depth per CPU worker
    gpu_load: &[usize],   // queue depth per GPU device
) -> Option<WorkerTarget> {
    if gpu_workers > 0 && segment_size >= gpu_threshold {
        // Choose GPU with lowest load
        let (idx, _) = gpu_load
            .iter()
            .enumerate()
            .min_by_key(|(_, load)| *load)
            .unwrap();
       Some(WorkerTarget::Gpu(idx))
    } 
    else if cpu_workers > 0 {
        // Choose CPU with lowest load
        let (idx, _) = cpu_load
            .iter()
            .enumerate()
            .min_by_key(|(_, load)| *load)
            .unwrap();
        Some(WorkerTarget::Cpu(idx))
    }
    else {
        None
    }
}

pub struct Scheduler {
    cpu_load: Vec<usize>, // queue depth per CPU worker
    gpu_load: Vec<usize>, // queue depth per GPU device
    gpu_threshold: usize, // segment size threshold for GPU dispatch
}

impl Scheduler {
    pub fn new(cpu_workers: usize, gpu_workers: usize, gpu_threshold: usize) -> Self {
        Scheduler {
            cpu_load: vec![0; cpu_workers],
            gpu_load: vec![0; gpu_workers],
            gpu_threshold,
        }
    }

    /// Dispatch a segment to CPU or GPU based on size and current load
    pub fn dispatch(&mut self, segment_size: usize) -> WorkerTarget {
        if !self.gpu_load.is_empty() && segment_size >= self.gpu_threshold {
            // Choose GPU with lowest load
            let (idx, _) = self.gpu_load
                .iter()
                .enumerate()
                .min_by_key(|(_, load)| *load)
                .unwrap();
            self.gpu_load[idx] += 1; // increment load
            WorkerTarget::Gpu(idx)
        } else {
            // Choose CPU with lowest load
            let (idx, _) = self.cpu_load
                .iter()
                .enumerate()
                .min_by_key(|(_, load)| *load)
                .unwrap();
            self.cpu_load[idx] += 1; // increment load
            WorkerTarget::Cpu(idx)
        }
    }

    /// Mark a worker as finished with a segment
    pub fn complete(&mut self, target: WorkerTarget) {
        match target {
            WorkerTarget::Cpu(idx) => {
                if self.cpu_load[idx] > 0 {
                    self.cpu_load[idx] -= 1;
                }
            }
            WorkerTarget::Gpu(idx) => {
                if self.gpu_load[idx] > 0 {
                    self.gpu_load[idx] -= 1;
                }
            }
        }
    }
}
