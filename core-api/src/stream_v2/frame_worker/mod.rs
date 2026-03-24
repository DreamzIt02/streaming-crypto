// ## 📦 `src/stream_v2/frame_worker/mod.rs`

//! Frame-level workers for stream_v2.
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
pub mod encrypt;
pub mod decrypt;

pub use encrypt:: {
    EncryptFrameWorker1,
};

pub use decrypt:: {
    DecryptFrameWorker1,
};

pub use types::{
    FrameInput,
    EncryptedFrame,
    DecryptedFrame,
    FrameWorkerError,
};