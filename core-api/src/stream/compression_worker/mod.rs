// ## 📦 `src/stream/compression_worker/mod.rs`

//! Wire compression for stream.
//!
//! Responsibilities:
//! - Compress segments into a canonical byte layout
//! - Decompress segments with strict validation
//!
//! Non-responsibilities:
//! - Cryptography
//! - IO
//! - Parallelism

pub mod types;
pub mod worker;
pub mod worker_cpu;
pub mod worker_gpu;

pub use types::{
    CompressionBackend,
    CodecInfo,
    CompressionWorkerError,
};
pub use worker::{
    make_backend,
};
pub use worker_cpu::{
    CpuCompressionBackend
};
pub use worker_gpu::{
    GpuCompressionBackend
};