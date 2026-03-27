// # 📂 `tests/test_segment_worker_counters.rs`

#[cfg(test)]
mod tests {

    use bytes::Bytes;
    use crossbeam::channel::{self, bounded};
    use core_api::{crypto::DigestAlg, segmenting::SegmentHeader, stream::{frame_worker::{EncryptedFrame, FrameInput}, segmenting::types::SegmentFlags}, telemetry::{StageTimes, TelemetryCounters}};
    use core_api::v3::{pipeline::Monitor, segment_worker::{SegmentInput, enc_helpers::process_encrypt_segment_3}};

    fn setup_channels() -> (
        channel::Sender<FrameInput>,
        channel::Receiver<EncryptedFrame>
    ) {
        // In real tests, we’d spawn a dummy frame worker.
        // For counters-only tests, we can simulate by sending back fake EncryptedFrames.
        let (frame_tx, frame_rx) = bounded::<FrameInput>(10);
        let (out_tx, out_rx) = bounded::<EncryptedFrame>(10);

        // Spawn a dummy responder thread that echoes back fake frames
        std::thread::spawn(move || {
            while let Ok(frame_in) = frame_rx.recv() {
                let fake_frame = EncryptedFrame {
                    segment_index: frame_in.segment_index,
                    frame_type: frame_in.frame_type,
                    frame_index: frame_in.frame_index,
                    wire: Bytes::from(vec![0u8; 8]), // fake wire length
                    stage_times: StageTimes::default(),
                    ct_range: 0..8
                };
                let _ = out_tx.send(fake_frame);
            }
        });

        (frame_tx, out_rx)
    }

    #[test]
    fn test_empty_final_segment_counters() {
        let (frame_tx, out_rx) = setup_channels();
        let input = SegmentInput {
            bytes: Bytes::new(),
            index: 0,
            flags: SegmentFlags::FINAL_SEGMENT,
            header: SegmentHeader::default(),
        };
        // Create monitor
        let (monitor, _monitor_rx) = Monitor::new(vec![], vec![]);

        let result = process_encrypt_segment_3(&input, 16, DigestAlg::Sha256, &frame_tx, &out_rx, monitor)
            .expect("should succeed");

        // Empty final segment should have all counters = 0
        assert_eq!(result.counters, TelemetryCounters::default());
    }

    #[test]
    fn test_single_frame_segment_counters() {
        let (frame_tx, out_rx) = setup_channels();
        let input = SegmentInput {
            index: 1,
            bytes: Bytes::from(vec![42u8; 16]), // one frame
            flags: SegmentFlags::empty(),
            header: SegmentHeader::default(),
        };
        // Create monitor
        let (monitor, _monitor_rx) = Monitor::new(vec![], vec![]);

        let result = process_encrypt_segment_3(&input, 16, DigestAlg::Sha256, &frame_tx, &out_rx, monitor)
            .expect("should succeed");

        let counters = result.counters;

        // Header always counted
        assert_eq!(counters.frames_header, 1);
        // One data frame
        assert_eq!(counters.frames_data, 1);
        // One digest frame
        assert_eq!(counters.frames_digest, 1);
        // One terminator frame
        assert_eq!(counters.frames_terminator, 1);

        // Overhead includes header + digest + terminator + per-frame header
        assert!(counters.bytes_overhead > 0);
        // Ciphertext length should be nonzero
        assert!(counters.bytes_ciphertext > 0);
    }

    #[test]
    fn test_multi_frame_segment_counters() {
        let (frame_tx, out_rx) = setup_channels();
        let input = SegmentInput {
            bytes: Bytes::from(vec![7u8; 64]), // multiple frames
            index: 2,
            flags: SegmentFlags::empty(),
            header: SegmentHeader::default(),
        };
        // Create monitor
        let (monitor, _monitor_rx) = Monitor::new(vec![], vec![]);

        let result = process_encrypt_segment_3(&input, 16, DigestAlg::Sha256, &frame_tx, &out_rx, monitor)
            .expect("should succeed");

        let counters = result.counters;

        // Header always counted
        assert_eq!(counters.frames_header, 1);
        // Should equal ceil(64/16) = 4 frames
        assert_eq!(counters.frames_data, 4);
        // Digest + terminator always present
        assert_eq!(counters.frames_digest, 1);
        assert_eq!(counters.frames_terminator, 1);

        // Ciphertext length should scale with frame count
        assert!(counters.bytes_ciphertext >= 2 * 16);
    }

    #[test]
    fn test_merge_counters() {
        let mut c1 = TelemetryCounters::default();
        c1.frames_header = 1;
        c1.bytes_ciphertext = 100;

        let mut c2 = TelemetryCounters::default();
        c2.frames_data = 2;
        c2.bytes_ciphertext = 50;

        c1.merge(&c2);

        assert_eq!(c1.frames_header, 1);
        assert_eq!(c1.frames_data, 2);
        assert_eq!(c1.bytes_ciphertext, 150);
    }
}
