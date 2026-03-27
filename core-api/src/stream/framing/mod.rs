// ## 📦 `src/stream/framing/mod.rs`

//! Wire framing for stream.
//!
//! Responsibilities:
//! - Define frame headers and records
//! - Encode frames into a canonical byte layout
//! - Decode frames with strict validation
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
    FrameHeader,
    FrameView,
    FrameType,
    FrameError,
};