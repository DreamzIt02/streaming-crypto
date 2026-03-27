// ## 📦 `src/stream/frame_worker/mod.rs`

//! Frame-level workers for stream.
//!
//! Responsibilities:
//! - Encrypt individual frames
//! - Decrypt individual frames
//! - Build AAD
//! - Derive nonces
//! - Encode / decode framing
//!
//! Non-responsibilities:
//! - IO
//! - Threading
//! - Ordering
//! - Segment aggregation

pub mod types;

pub use types::{
    FrameInput,
    EncryptedFrame,
    DecryptedFrame,
    FrameWorkerError,
};