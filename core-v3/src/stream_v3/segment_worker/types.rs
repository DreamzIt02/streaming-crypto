// ## 📦 `src/stream_v3/segment_worker/types.rs`

use bytes::Bytes;

use core_api::stream::{segment_worker::EncryptedSegment, segmenting::{SegmentHeader, types::SegmentFlags}};

/// `SegmentInput` is the “raw” form: just plaintext frames.
/// Input from reader stage (plaintext)
#[derive(Debug, Clone)]
pub struct SegmentInput {
    pub index: u32,  // u32 matches our frame header type
    pub bytes: Bytes, // 🔥 zero-copy shared
    pub flags: SegmentFlags, // 🔥 final segment, or other flags bit input from pipeline
    // pub stage_times: StageTimes,
    pub header: SegmentHeader,
}

// Convert EncryptedSegment → SegmentInput
impl From<EncryptedSegment> for SegmentInput {
    fn from(seg: EncryptedSegment) -> Self {
        SegmentInput {
            index: seg.header.segment_index(),
            bytes: seg.wire,
            flags: seg.header.flags(),
            header: seg.header,
        }
    }
}
