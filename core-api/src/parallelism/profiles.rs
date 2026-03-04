// ## 3. `src/parallelism/profiles.rs`

use sysinfo::{System};
use tracing::debug;
use crate::{headers::{HeaderV1, Strategy}, types::StreamError};

pub const GPU_THRESHOLD: usize = 4 * 1024 * 1024; // 4 MB

#[derive(Debug, Copy, Clone)]
pub enum GpuBackend {
    None,
    Cuda,
    Wgpu,
    OpenCL,
}

#[derive(Debug, Clone)]
pub struct GpuInfo {
    pub count: usize,
    pub backend: GpuBackend,
    pub device_names: Vec<String>,
}

#[cfg(feature = "cuda")]
fn detect_cuda_devices() -> (usize, Vec<String>) {
    // cudarc automatically queries available devices
    let devices = cudarc::driver::CudaDevice::devices().unwrap_or_default();
    let count = devices.len();
    let names: Vec<String> = devices
        .iter()
        .map(|d| d.name().unwrap_or_else(|_| "Unknown CUDA device".to_string()))
        .collect();

    if count == 0 {
        debug!("[GPU DETECT] No CUDA devices available");
    }
    (count, names)
}

#[cfg(feature = "metal")]
fn detect_wgpu_devices() -> (usize, Vec<String>) {
    let instance = wgpu::Instance::default();
    // enumerate_adapters returns a Future in wgpu 0.28
    let adapters: Vec<wgpu::Adapter> =
        pollster::block_on(instance.enumerate_adapters(wgpu::Backends::all()));

    let count = adapters.len();
    let names: Vec<String> = adapters
        .iter()
        .map(|a| a.get_info().name.clone())
        .collect();

    (count, names)
}

#[cfg(feature = "opencl")]
fn detect_opencl_devices() -> (usize, Vec<String>) {
    use opencl3::platform::get_platforms;
    use opencl3::device::{get_all_devices, Device, CL_DEVICE_TYPE_ALL};

    let mut names = Vec::new();
    let mut count = 0;

    match get_platforms() {
        Ok(platforms) => {
            for _p in platforms {
                // get_all_devices only takes a device type
                if let Ok(device_ids) = get_all_devices(CL_DEVICE_TYPE_ALL) {
                    count += device_ids.len();
                    names.extend(device_ids.iter().map(|&id| {
                        let dev = Device::new(id);
                        dev.name().unwrap_or_else(|_| "Unknown OpenCL device".to_string())
                    }));
                }
            }
        }
        Err(_) => debug!("[GPU DETECT] No OpenCL platforms available"),
    }
    (count, names)
}

pub fn detect_gpu_info() -> GpuInfo {
    // CUDA
    #[cfg(feature = "cuda")]
    {
        let (count, names) = detect_cuda_devices();
        if count > 0 {
            eprintln!( "[GPU DETECT] CUDA devices found: {} ({})", count, names.join(", ") );
            return GpuInfo {
                count,
                backend: GpuBackend::Cuda,
                device_names: names,
            };
        }
    }

    // Vulkan/Metal/DX via wgpu
    #[cfg(feature = "metal")]
    {
        // let count = pollster::block_on(detect_wgpu_count());
        let (count, names) = detect_wgpu_devices();
        if count > 0 {
            eprintln!( "[GPU DETECT] wgpu adapters found: {} ({})", count, names.join(", ") );
            return GpuInfo {
                count,
                backend: GpuBackend::Wgpu,
                device_names: names,
            };
        }
    }

    // OpenCL
    #[cfg(feature = "opencl")]
    {
        let (count, names) = detect_opencl_devices();
        if count > 0 {
            eprintln!( "[GPU DETECT] OpenCL adapters found: {} ({})", count, names.join(", ") );
            return GpuInfo {
                count,
                backend: GpuBackend::OpenCL,
                device_names: names,
            };
        }
    }

    // Fallback
    eprintln!("[GPU DETECT] No GPU devices found");
    GpuInfo {
        count: 0,
        backend: GpuBackend::None,
        device_names: Vec::new(),
    }
}

/// Parallelism configuration
#[derive(Debug, Clone)]
pub struct ParallelismConfig {
    cpu_workers: usize, 
    gpu_workers: usize, 
    mem_fraction: f64,
    hard_cap: usize,
}

impl Default for ParallelismConfig {
    fn default() -> Self {
        Self {
            cpu_workers: 1,
            gpu_workers: 0, 
            mem_fraction: 0.2, // 20% of free memory 
            hard_cap: 4, // Max in-flights segments limit on dynamic
        }
    }
}
impl ParallelismConfig {
    pub fn new(cpu_workers: usize, gpu_workers: usize, mem_fraction: f64, hard_cap: usize) -> Self {
        Self {
            cpu_workers,
            gpu_workers,
            mem_fraction,
            hard_cap,
        }
    }
}

/// Parallelism configuration
#[derive(Debug, Clone)]
pub struct HybridParallelismProfile {
    cpu_workers: usize,
    gpu_workers: usize,
    inflight_segments: usize,
    gpu_threshold: usize,
    gpu: Option<GpuInfo>,
}

impl HybridParallelismProfile {
    /// Controlled constructor
    fn new(max_segment_size: u32, opts: ParallelismConfig) -> Self {
        let gpu = detect_gpu_info();

        // enforce sane limits
        let cpu_workers = opts.cpu_workers.clamp(1, num_cpus::get().saturating_sub(1));
        let gpu_workers = opts.gpu_workers.clamp(0, gpu.count);

        // --- memory based inflight_segments calculation ---
        let mut sys = System::new();
        sys.refresh_memory();

        // available memory in bytes
        let avail_bytes = sys.available_memory() * 1024; // KiB → bytes

        // fraction capped to 0.75 (use at most 75% of available memory)
        let fraction = opts.mem_fraction.min(0.75);
        let usable_bytes = (avail_bytes as f64 * fraction) as u64;

        // cast max_segment_size to u64 before division
        let max_segments = (usable_bytes / max_segment_size as u64).max(1) as usize;

        // clamp against hard_cap
        let inflight_segments_cap = opts.hard_cap.clamp(1, max_segments);
        // Derive inflight segments from worker count
        let inflight_cpus = inflight_segments_cap.min(cpu_workers * 4);
        let inflight_gpus = inflight_segments_cap.min(gpu_workers * 4);
        let inflight_segments = inflight_cpus.max(inflight_gpus) as usize;

        Self {
            cpu_workers,
            gpu_workers,
            inflight_segments,
            gpu_threshold: GPU_THRESHOLD,
            gpu: Some(gpu),
        }
    }

    pub fn with_strategy(
        strategy: Strategy,
        max_segment_size: u32,
        config: Option<ParallelismConfig>,
    ) -> Result<Self, StreamError> {
        let opts = config.unwrap_or_default();
        match strategy {
            Strategy::Auto => Ok(Self::dynamic(max_segment_size)),
            Strategy::Sequential => Ok(Self::single_threaded()),
            Strategy::Parallel => Ok(Self::new(max_segment_size, opts)),
        }
    }

    pub fn from_stream_header(
        header: HeaderV1,
        config: Option<ParallelismConfig>,
    ) -> Result<Self, StreamError> {
        let max_segment_size = header.chunk_size;
        let strategy = Strategy::from(header.strategy).map_err(StreamError::Header)?;

        Self::with_strategy(strategy, max_segment_size, config)
    }

    /// Read-only accessors
    pub fn cpu_workers(&self) -> usize {
        self.cpu_workers
    }

    pub fn gpu_workers(&self) -> usize {
        self.gpu_workers
    }

    pub fn inflight_segments(&self) -> usize {
        self.inflight_segments
    }

    pub fn gpu_threshold(&self) -> usize {
        self.gpu_threshold
    }

    pub fn gpu(&self) -> Option<GpuInfo> {
        self.gpu.clone()
    }
    
    pub fn single_threaded() -> Self {
        let gpu = detect_gpu_info();
        Self {
            cpu_workers: 1,
            gpu_workers: 1.clamp(0, gpu.count),
            inflight_segments: 1,
            gpu_threshold: GPU_THRESHOLD,
            gpu: Some(gpu),
        }
    }
    // * On a machine with 16 cores and 16 GB free RAM:
    // * `worker_count = 15`
    // * `budget = 8 GB` (50% of 16 GB)
    // * `max_segment_size = 32 MB`
    // * `max_segments = 8192 MB / 32 MB = 256`
    // * With `hard_cap = 64`, we get `inflight_segments = 64`.
    pub fn semi_dynamic(max_segment_size: u32, mem_fraction: f64, hard_cap: usize) -> Self {
        // * Hyperthreads do not double AES throughput.
        // * Physical cores matter.
        let cores = num_cpus::get().saturating_sub(2);
        let cpu_workers = cores.max(1);

        let mut sys = sysinfo::System::new_all();
        sys.refresh_memory();
        let avail_bytes = sys.available_memory() * 1024;
        let budget = (avail_bytes as f64 * mem_fraction) as u32;
        let max_segments = budget / max_segment_size;

        let gpu = detect_gpu_info();
        let gpu_workers = gpu.count;

        debug!(
            "[PROFILE] cpu_workers={}, gpu_workers={}, inflight_segments={}",
            cpu_workers,
            gpu_workers,
            max_segments.min(hard_cap as u32)
        );

        Self {
            cpu_workers,
            gpu_workers,
            inflight_segments: max_segments.min(hard_cap as u32) as usize,
            gpu_threshold: GPU_THRESHOLD,
            gpu: Some(gpu),
        }
    }

    // * On a machine with 16 cores and 16 GB free RAM:
    // * `worker_count = 15`
    // * `budget = 8 GB` (50% of 16 GB)
    // * `max_segment_size = 32 MB`
    // * `max_segments = 8192 MB / 32 MB = 256`
    // * With `hard_cap = 64`, we get `inflight_segments = 64`.
    pub fn dynamic(max_segment_size: u32) -> Self {
        let cores = num_cpus::get().saturating_sub(2);
        let cpu_workers = cores.max(1);

        let mut sys = sysinfo::System::new_all();
        sys.refresh_memory();
        let avail_bytes = sys.available_memory() * 1024;

        // Leave 25% headroom for OS and other processes
        let budget = (avail_bytes as f64 * 0.75) as u32;
        let max_segments = budget / max_segment_size;

        let gpu = detect_gpu_info();
        let gpu_workers = gpu.count;

        // Derive inflight segments from worker count
        let inflight_cpus = max_segments.min(cpu_workers as u32 * 4);
        let inflight_gpus = max_segments.min(gpu_workers as u32 * 4);
        let inflight_segments = inflight_cpus.max(inflight_gpus) as usize;

        debug!(
            "[PROFILE] cpu_workers={}, gpu_workers={}, inflight_segments={}",
            cpu_workers, gpu_workers, inflight_segments
        );

        Self {
            cpu_workers,
            gpu_workers,
            inflight_segments,
            gpu_threshold: GPU_THRESHOLD,
            gpu: Some(gpu),
        }
    }

}
