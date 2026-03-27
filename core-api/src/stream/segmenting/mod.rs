// ## 📦 `src/stream/segmenting/mod.rs`

//! Wire segmenting for stream.
//!
//! Responsibilities:
//! - Define segment headers and records
//! - Encode segments into a canonical byte layout
//! - Decode segments with strict validation
//!
//! Non-responsibilities:
//! - Cryptography
//! - Compression
//! - IO
//! - Parallelism

pub mod types;
pub mod encode;
pub mod decode;

pub use types::{
    SegmentHeader,
};
pub use encode::{
    encode_segment,
};
pub use decode::{
    decode_segment_header,
    decode_segment,
};