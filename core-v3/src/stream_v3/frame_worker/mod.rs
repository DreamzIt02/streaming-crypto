// ## 📦 `src/stream_v3/frame_worker/mod.rs`

//! Frame-level workers for stream_v3.
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

pub mod encrypt;
pub mod decrypt;

pub use encrypt:: {
    EncryptFrameWorker3,
};

pub use decrypt:: {
    DecryptFrameWorker3,
};
