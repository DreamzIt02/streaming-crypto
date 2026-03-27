// ## 📦 `src/stream_v2/compression_worker/mod.rs`

//! Wire compression for stream_v2.
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

pub use worker::{
    run_compression_worker,
    run_decompression_worker,
};
