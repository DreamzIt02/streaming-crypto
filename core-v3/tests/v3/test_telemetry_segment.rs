// # 📂 `tests/test_telemetry_segment.rs`
// # 🧪 Comprehensive Test Suite for Segment Worker Telemetry
//
// Guarantees:
// ✔ Correct handling of empty segments (final vs non‑final)
// ✔ Proper frame segmentation (single, multi‑frame)
// ✔ Telemetry counters and stage times updated correctly
// ✔ Error detection for invalid inputs and digest mismatches
// ✔ Decrypt pipeline symmetry with encrypt pipeline
// ✔ Counter merging correctness
//
// If any test fails, it means:
// * Segment framing invariants were broken,
// * Digest verification regressed,
// * Or telemetry accounting became inconsistent.

#[cfg(test)]
mod tests {
    use std::time::Duration;
    use bytes::Bytes;
    use crossbeam::channel::{Receiver, Sender, bounded, unbounded};
    use core_api::{
        crypto::DigestAlg, headers::HeaderV1, stream_v2::{
            frame_worker::{
                DecryptedFrame, EncryptedFrame, FrameInput,
            },
            segment_worker::
                EncryptedSegment
            ,
            segmenting::{SegmentHeader, types::SegmentFlags},
        }, 
        telemetry::{Stage, StageTimes, TelemetryCounters}, types::StreamError
    };
    use core_v3::stream_v3::{
        frame_worker::{
            decrypt::DecryptFrameWorker3, encrypt::EncryptFrameWorker3
        }, pipeline::Monitor, segment_worker::{
            SegmentInput, 
            dec_helpers::process_decrypt_segment_3, enc_helpers::process_encrypt_segment_3,
        }
    };
    
    // ## 1️⃣ Helpers
    fn make_encrypt_channels() -> (
        Sender<FrameInput>,
        Receiver<EncryptedFrame>,
    ) {
        let (frame_tx, frame_rx) = bounded::<FrameInput>(4);
        let (out_tx, out_rx) = unbounded::<EncryptedFrame>();

        let header = HeaderV1::test_header();
        let session_key = vec![0u8; 32];

        let (monitor_enc, _monitor_rx_enc) = Monitor::new(
            vec![],
            vec![],
        );

        let fw = EncryptFrameWorker3::new(header, &session_key, monitor_enc);
        std::thread::spawn(move || fw.run(frame_rx, out_tx));

        (frame_tx, out_rx)
    }

    fn make_decrypt_channels() -> (
        Sender<Bytes>,
        Receiver<DecryptedFrame>,
    ) {
        let (frame_tx, frame_rx) = bounded::<Bytes>(4);
        let (out_tx, out_rx) = unbounded::<DecryptedFrame>();

        let header = HeaderV1::test_header();
        let session_key = vec![0u8; 32];
        
        let (monitor_dec, _monitor_rx_dec) = Monitor::new(
            vec![],
            vec![],
        );

        let fw = DecryptFrameWorker3::new(header, &session_key, monitor_dec);
        std::thread::spawn(move || fw.run(frame_rx, out_tx));

        (frame_tx, out_rx)
    }

    fn build_fake_encrypted_segment(alg: DigestAlg) -> Option<EncryptedSegment> {
        let plaintext = Bytes::from_static(b"hello world telemetry test");
        let input = SegmentInput {
            bytes: plaintext.clone(),
            index: 42,
            flags: SegmentFlags::empty(),
            header: SegmentHeader::default(),
        };

        let (frame_tx, frame_rx) = bounded::<FrameInput>(4);
        let (out_tx, out_rx) = unbounded::<EncryptedFrame>();
        let header = HeaderV1::test_header();
        let session_key = vec![0u8; 32];

        let (monitor_enc, _monitor_rx_enc) = Monitor::new(
            vec![],
            vec![],
        );

        let fw = EncryptFrameWorker3::new(header, &session_key, monitor_enc.clone());
        std::thread::spawn(move || fw.run(frame_rx, out_tx));

        process_encrypt_segment_3(&input, 16, alg, &frame_tx, &out_rx, monitor_enc)
    }

    // ## 2️⃣ Encrypt Segment Tests
    #[test]
    fn encrypt_empty_final_segment() {
        let (frame_tx, out_rx) = make_encrypt_channels();
        
        let (monitor_enc, _monitor_rx_enc) = Monitor::new(
            vec![],
            vec![],
        );

        let input = SegmentInput {
            index: 0,
            bytes: Bytes::new(),
            flags: SegmentFlags::FINAL_SEGMENT,
            header: SegmentHeader::default(),
        };
        let result = process_encrypt_segment_3(&input, 1024, DigestAlg::Blake3, &frame_tx, &out_rx, monitor_enc).unwrap();
        assert!(result.wire.is_empty());
        assert_eq!(result.counters.frames_data, 0);
        assert_eq!(result.stage_times.total(), Duration::ZERO);
    }

    #[test]
    fn encrypt_single_frame_segment() {
        let (frame_tx, out_rx) = make_encrypt_channels();
        
        let (monitor_enc, _monitor_rx_enc) = Monitor::new(
            vec![],
            vec![],
        );

        let input = SegmentInput {
            index: 1,
            bytes: Bytes::from_static(b"hello world"),
            flags: SegmentFlags::empty(),
            header: SegmentHeader::default(),
        };
        let result = process_encrypt_segment_3(&input, 64, DigestAlg::Blake3, &frame_tx, &out_rx, monitor_enc).unwrap();
        assert_eq!(result.counters.frames_data, 1);
        assert!(result.counters.bytes_ciphertext > 0);
        assert!(result.stage_times.get(Stage::Encrypt) > Duration::ZERO);
        assert!(result.stage_times.get(Stage::Digest) > Duration::ZERO);
        assert!(result.stage_times.get(Stage::Write) > Duration::ZERO);
    }

    #[test]
    fn encrypt_multi_frame_segment() {
        let (frame_tx, out_rx) = make_encrypt_channels();
        let (monitor_enc, _monitor_rx_enc) = Monitor::new(
            vec![],
            vec![],
        );

        let big_payload = vec![42u8; 4096];
        let input = SegmentInput {
            index: 2,
            bytes: Bytes::from(big_payload),
            flags: SegmentFlags::empty(),
            header: SegmentHeader::default(),
        };
        let result = process_encrypt_segment_3(&input, 1024, DigestAlg::Blake3, &frame_tx, &out_rx, monitor_enc).unwrap();
        assert_eq!(result.counters.frames_data, 4);
        assert!(result.counters.bytes_ciphertext >= 4096);
        assert!(result.stage_times.get(Stage::Encode) > Duration::ZERO);
        assert!(result.stage_times.get(Stage::Encrypt) > Duration::ZERO);
        assert!(result.stage_times.get(Stage::Digest) > Duration::ZERO);
        assert!(result.stage_times.get(Stage::Write) > Duration::ZERO);
    }

    #[test]
    fn encrypt_invalid_empty_non_final_segment() {
        let (frame_tx, out_rx) = make_encrypt_channels();
        let (monitor_enc, _monitor_rx_enc) = Monitor::new(
            vec![],
            vec![],
        );
        let input = SegmentInput {
            index: 3,
            bytes: Bytes::new(),
            flags: SegmentFlags::empty(),
            header: SegmentHeader::default(),
        };
        let result = process_encrypt_segment_3(&input, 1024, DigestAlg::Blake3, &frame_tx, &out_rx, monitor_enc);
        assert!(result.is_none());

        let mut saw_error = false;
        while let Ok(event) = _monitor_rx_enc.recv() {
            match event {
                Err(_) => {
                    saw_error = true;
                    break;
                }
                Ok(_) => {} // telemetry snapshot
            }
        }
        assert!(saw_error, "expected SegmentWorker error");

    }

    // ## 3️⃣ Decrypt Segment Tests
    #[test]
    fn decrypt_empty_final_segment() {
        let (frame_tx, out_rx) = make_decrypt_channels();
        let (monitor_dec, _monitor_rx_dec) = Monitor::new(
            vec![],
            vec![],
        );

        let header = SegmentHeader::new(&Bytes::new(), 0, 0, 0, 0, SegmentFlags::FINAL_SEGMENT);
        let input = SegmentInput { header, bytes: Bytes::new(), index: header.segment_index(), flags: header.flags() };
        let result = process_decrypt_segment_3(&input, &DigestAlg::Sha256, &frame_tx, &out_rx, monitor_dec).unwrap();
        assert_eq!(result.bytes.len(), 0);
        assert_eq!(result.counters, TelemetryCounters::default());
        assert_eq!(result.stage_times, StageTimes::default());
    }

    #[test]
    fn decrypt_invalid_empty_non_final_segment() {
        let (frame_tx, out_rx) = make_decrypt_channels();
        let (monitor_dec, monitor_rx_dec) = Monitor::new(
            vec![],
            vec![],
        );
        let header = SegmentHeader::new(&Bytes::new(), 1, 0, 0, DigestAlg::Sha256 as u16, SegmentFlags::empty());
        let input = SegmentInput { header, bytes: Bytes::new(), index: header.segment_index(), flags: header.flags() };
        let result = process_decrypt_segment_3(&input, &DigestAlg::Sha256, &frame_tx, &out_rx, monitor_dec);
        assert!(result.is_none());

        let mut saw_error = false;
        while let Ok(event) = monitor_rx_dec.recv() {
            match event {
                Err(StreamError::SegmentWorker(_)) => {
                    saw_error = true;
                    break;
                }
                Ok(_) => {}, // telemetry snapshot
                Err(_) => {
                    // Ignore
                }
            }
        }
        assert!(saw_error, "expected SegmentWorker error");

    }

    #[test]
    fn decrypt_digest_mismatch() {
        let (frame_tx, out_rx) = make_decrypt_channels();
        let (monitor_dec, monitor_rx_dec) = Monitor::new(
            vec![],
            vec![],
        );

        let bogus_wire = Bytes::from_static(&[0x01, 0x02, 0x03]);
        let header = SegmentHeader::new(
            &bogus_wire,
            2,
            bogus_wire.len() as u32,
            1,
            DigestAlg::Sha256 as u16,
            SegmentFlags::empty(),
        );
        let input = SegmentInput { header, bytes: bogus_wire, index: header.segment_index(), flags: header.flags() };

        let result = process_decrypt_segment_3(&input, &DigestAlg::Sha256, &frame_tx, &out_rx, monitor_dec);
        assert!(result.is_none());

        let mut saw_error = false;
        while let Ok(event) = monitor_rx_dec.recv() {
            match event {
                Err(StreamError::SegmentWorker(_)) => {
                    saw_error = true;
                    break;
                }
                Ok(_) => {}, // telemetry snapshot
                Err(_) => {
                    // Ignore
                }
            }
        }
        assert!(saw_error, "expected SegmentWorker error");
    }

    #[test]
    fn decrypt_successful_updates_counters() {
        let (frame_tx, out_rx) = make_decrypt_channels();
        let (monitor_dec, _monitor_rx_dec) = Monitor::new(
            vec![],
            vec![],
        );

        let alg = DigestAlg::Blake3;
        let encrypted_segment = build_fake_encrypted_segment(alg).unwrap();
        let input = SegmentInput::from(encrypted_segment);

        let result = process_decrypt_segment_3(&input, &alg, &frame_tx, &out_rx, monitor_dec).unwrap();

        // Telemetry counters should reflect data, digest, terminator
        assert!(result.counters.bytes_compressed > 0);
        assert!(result.counters.frames_digest > 0);
        assert!(result.counters.frames_terminator > 0);

        // Stage times should have nonzero durations
        assert!(result.stage_times.get(Stage::Decode)   > Duration::ZERO);
        assert!(result.stage_times.get(Stage::Decrypt)  > Duration::ZERO);
        assert!(result.stage_times.get(Stage::Digest)   > Duration::ZERO);
        assert!(result.stage_times.get(Stage::Write)    > Duration::ZERO);
    }

    #[test]
    fn telemetry_merge_counters() {
        let mut c1 = TelemetryCounters::default();
        let mut c2 = TelemetryCounters::default();
        c1.bytes_plaintext = 100;
        c2.bytes_plaintext = 50;

        c1.merge(&c2);
        assert_eq!(c1.bytes_plaintext, 150);
    }
}
