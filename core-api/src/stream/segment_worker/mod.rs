// ## 📦 `src/stream/segment_worker/mod.rs`

//! Segment-level workers for stream.
//!
//! A segment is a *bounded batch of frames* processed independently.
//! Segment workers:
//! - fan-in frame workers
//! - preserve per-segment ordering
//! - emit a single contiguous wire blob
//!
//! They are:
//! - CPU-bound
//! - Stateless between segments
//! - Fully parallelizable

pub mod types;

pub use types::{
    EncryptSegmentInput,
    DecryptSegmentInput,
    EncryptedSegment,
    DecryptedSegment,
    EncryptContext,
    DecryptContext,
    SegmentWorkerError,
};
