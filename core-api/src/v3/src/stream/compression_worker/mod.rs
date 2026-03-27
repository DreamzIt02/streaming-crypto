// ## 📦 `src/stream_v3/compression_worker/mod.rs`

//! Wire compression for stream_v3.
//!
//! Responsibilities:
//! - Compress segments into a canonical byte layout
//! - Decompress segments with strict validation
//!
//! Non-responsibilities:
//! - Cryptography
//! - IO
//! - Parallelism

pub mod worker;

pub use worker::{
    run_compress_worker,
    run_decompress_worker,
};
