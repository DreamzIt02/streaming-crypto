// # 📂 src/stream/mod.rs

// ## 1️⃣ `mod.rs` — public façade + re-exports

//! stream — fully parallel, segment-based streaming encryption/decryption.
//!
//! This module exposes a **stable public API** for Rust, Python (via PyO3),
//! CLI tools, and services. Internals are strictly layered.


pub mod framing;
pub mod segmenting;
pub mod frame_worker;
pub mod segment_worker;
pub mod compression_worker;
pub mod core;
pub mod io;


pub use io::{
    InputSource,
    OutputSink,
};

pub use core::{
    MasterKey,
};
