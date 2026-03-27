// # 📂 src/stream_v3/mod.rs

// ## 1️⃣ `mod.rs` — public façade + re-exports

//! stream_v3 — fully parallel, segment-based streaming encryption/decryption.
//!
//! This module exposes a **stable public API** for Rust, Python (via PyO3),
//! CLI tools, and services. Internals are strictly layered.

pub mod compression_worker;
pub mod frame_worker;
pub mod segment_worker;
pub mod pipeline;
pub mod core;

pub use core::{
    ApiConfig, EncryptParams, DecryptParams,
    encrypt_stream_v3,
    decrypt_stream_v3,
};

