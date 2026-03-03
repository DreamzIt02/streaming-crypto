// ## src/compression/codecs/mod.rs

//! compression/codes/mod.rs
//! Streaming-safe compression and decompression.
//!
//! Industry notes:
//! - Deterministic per-chunk compression ensures reproducibility and parallel safety.
//! - Dictionaries must be explicitly declared and bound via header.dict_id.
//! - Registry resolves codec IDs to implementations.

pub mod auto;
pub mod deflate;
pub mod lz4;
pub mod zstd;

pub use auto::*;
pub use deflate::*;
pub use lz4::*;
pub use zstd::*;
