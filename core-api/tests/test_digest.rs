// # 🧪 Comprehensive Test Suite for `digest.rs`
//
// Guarantees:
// ✔ Round‑trip correctness for all algorithms
// ✔ Digest mismatch detection
// ✔ Wire format validation (length, format, algorithm IDs)
// ✔ Property‑based fuzz coverage
// ✔ Multi‑algorithm support (SHA‑2, SHA‑3, Blake3)
// ✔ Determinism (same input → same digest)
//
// If any test fails, it means:
// * Digest framing spec was broken,
// * A new algorithm was added unsafely,
// * Or a security guarantee regressed.

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use bytes::Bytes;
    use core_api::{
        crypto::{DigestAlg, SegmentDigestBuilder, DigestError, DigestFrame, SegmentDigestVerifier},
        stream_v2::{frame_worker::{FrameInput, FrameWorkerError}, framing::FrameType},
    };

    // ## 1️⃣ Helper
    fn run_roundtrip(alg: DigestAlg) {
        let segment_index = 42u32;
        let frames = vec![(0u32, b"hello".to_vec()), (1u32, b"world".to_vec())];
        let frame_count = frames.len() as u32;

        let mut builder = SegmentDigestBuilder::new(alg, segment_index, frame_count);
        for (idx, ct) in &frames {
            builder.update_frame(*idx, ct);
        }
        let digest_bytes = builder.finalize().unwrap();

        let frame = DigestFrame { algorithm: alg, digest: digest_bytes.clone() };
        let encoded = frame.encode();
        let decoded = DigestFrame::decode(&encoded).unwrap();
        assert_eq!(decoded.algorithm, alg);
        assert_eq!(decoded.digest, digest_bytes);

        let mut verifier = SegmentDigestVerifier::new(alg, segment_index, frame_count);
        for (idx, ct) in &frames {
            verifier.update_frame(*idx, ct);
        }
        let actual = verifier.finalize().unwrap();
        assert!(SegmentDigestVerifier::verify(actual, digest_bytes).is_ok());
    }

    // ## 2️⃣ Roundtrip Tests – All Algorithms
    #[test] fn sha256_roundtrip() { run_roundtrip(DigestAlg::Sha256); }
    #[test] fn sha512_roundtrip() { run_roundtrip(DigestAlg::Sha512); }
    #[test] fn sha3_256_roundtrip() { run_roundtrip(DigestAlg::Sha3_256); }
    #[test] fn sha3_512_roundtrip() { run_roundtrip(DigestAlg::Sha3_512); }
    #[test] fn blake3_roundtrip() { run_roundtrip(DigestAlg::Blake3); }

    // ## 3️⃣ Negative Cases
    #[test]
    fn tampered_ciphertext_detected() {
        let mut builder = SegmentDigestBuilder::new(DigestAlg::Sha256, 1, 1);
        builder.update_frame(0, b"original");
        let digest_bytes = builder.finalize().unwrap();

        let mut verifier = SegmentDigestVerifier::new(DigestAlg::Sha256, 1, 1);
        verifier.update_frame(0, b"tampered");
        let actual = verifier.finalize().unwrap();
        let result = SegmentDigestVerifier::verify(actual, digest_bytes);
        assert!(matches!(result, Err(DigestError::DigestMismatch { .. })));
    }

    #[test]
    fn invalid_digest_length_detected() {
        let fake_digest = vec![1, 2, 3, 4, 5];
        let frame = DigestFrame { algorithm: DigestAlg::Sha256, digest: fake_digest.clone() };
        let mut encoded = frame.encode();
        encoded[2] = 0x00; encoded[3] = 0x10; // corrupt length
        assert!(matches!(DigestFrame::decode(&encoded), Err(DigestError::InvalidLength { .. })));
    }

    #[test]
    fn invalid_format_detected() {
        let bad_bytes = vec![0x00, 0x01];
        assert!(matches!(DigestFrame::decode(&bad_bytes), Err(DigestError::InvalidFormat)));
    }

    // ## 4️⃣ Frame Validation
    fn make_digest_frame(alg: DigestAlg, digest: &[u8]) -> FrameInput {
        let digest_frame = DigestFrame { algorithm: alg, digest: digest.to_vec() };
        FrameInput {
            frame_type: FrameType::Digest,
            segment_index: 0,
            frame_index: 0,
            payload: Bytes::from(digest_frame.encode()),
        }
    }

    #[test] fn digest_frame_valid_sha256() { assert!(make_digest_frame(DigestAlg::Sha256, &[0xAA; 32]).validate().is_ok()); }
    #[test] fn digest_frame_valid_sha512() { assert!(make_digest_frame(DigestAlg::Sha512, &[0xBB; 64]).validate().is_ok()); }
    #[test] fn digest_frame_valid_blake3() { assert!(make_digest_frame(DigestAlg::Blake3, &[0xCC; 32]).validate().is_ok()); }

    #[test]
    fn digest_frame_too_short_fails() {
        let frame = FrameInput { frame_type: FrameType::Digest, segment_index: 0, frame_index: 0, payload: Bytes::from(vec![0x01, 0x02]) };
        let err = frame.validate().unwrap_err();
        assert!(matches!(err, FrameWorkerError::InvalidInput(msg) if msg.contains("too short")));
    }

    #[test]
    fn digest_frame_unknown_algorithm_fails() {
        let mut buf = Vec::new();
        buf.extend_from_slice(&0x9999u16.to_be_bytes());
        buf.extend_from_slice(&(4u16).to_be_bytes());
        buf.extend_from_slice(&[0x11, 0x22, 0x33, 0x44]);
        assert!(matches!(DigestFrame::decode(&buf), Err(DigestError::UnknownAlgorithm { .. })));
    }

    // ## 5️⃣ Property‑Based Tests
    proptest! {
        #[test]
        fn prop_digest_roundtrip_agreement(
            segment_index in any::<u32>(),
            frame_count in 1u32..5,
            frames in proptest::collection::vec(proptest::collection::vec(any::<u8>(), 0..64), 1..5),
            alg in proptest::sample::select(&[DigestAlg::Sha256, DigestAlg::Sha512, DigestAlg::Sha3_256, DigestAlg::Sha3_512, DigestAlg::Blake3])
        ) {
            let mut builder = SegmentDigestBuilder::new(alg, segment_index, frame_count);
            for (i, ct) in frames.iter().enumerate() { builder.update_frame(i as u32, ct); }
            let digest_bytes = builder.finalize().unwrap();

            let frame = DigestFrame { algorithm: alg, digest: digest_bytes.clone() };
            let encoded = frame.encode();
            let decoded = DigestFrame::decode(&encoded).unwrap();
            prop_assert_eq!(decoded.algorithm, alg);
            prop_assert_eq!(decoded.digest, digest_bytes.clone());

            let mut verifier = SegmentDigestVerifier::new(alg, segment_index, frame_count);
            for (i, ct) in frames.iter().enumerate() { verifier.update_frame(i as u32, ct); }
            let actual = verifier.finalize().unwrap();
            prop_assert!(SegmentDigestVerifier::verify(actual, digest_bytes).is_ok());
        }
    }

    proptest! {
        #[test]
        fn prop_tampered_ciphertext_detected(
            segment_index in any::<u32>(),
            ciphertext in proptest::collection::vec(any::<u8>(), 1..64)
        ) {
            let frame_count = 1u32;
            let mut builder = SegmentDigestBuilder::new(DigestAlg::Sha256, segment_index, frame_count);
            builder.update_frame(0, &ciphertext);
            let digest_bytes = builder.finalize().expect("Failed to finalize digest");

            // Tamper ciphertext by flipping a bit
            let mut tampered = ciphertext.clone();
            tampered[0] ^= 0xFF;

            let mut verifier = SegmentDigestVerifier::new(DigestAlg::Sha256, segment_index, frame_count);
            verifier.update_frame(0, &tampered);

            let actual = verifier.finalize().expect("Digest finalize failed");
            let result = SegmentDigestVerifier::verify(actual, digest_bytes);

            // ✅ No external crate needed
            if let Err(DigestError::DigestMismatch { have, need }) = result {
                prop_assert!(have != need);
            } else {
                prop_assert!(false, "Expected DigestMismatch error");
            }
        }
    }

    proptest! {
        #[test]
        fn prop_invalid_frame_encoding_detected(
            alg_id in any::<u16>(),
            digest in proptest::collection::vec(any::<u8>(), 0..32)
        ) {
            let mut encoded = Vec::new();
            encoded.extend_from_slice(&alg_id.to_be_bytes());
            encoded.extend_from_slice(&(digest.len() as u16 + 5).to_be_bytes()); // wrong length
            encoded.extend_from_slice(&digest);
            prop_assert!(DigestFrame::decode(&encoded).is_err());
        }
    }

    // ## 6️⃣ Determinism
    #[test]
    fn digest_is_deterministic() {
        let mut a = SegmentDigestBuilder::new(DigestAlg::Sha256, 1, 1);
        let mut b = SegmentDigestBuilder::new(DigestAlg::Sha256, 1, 1);
        a.update_frame(0, b"x");
        b.update_frame(0, b"x");

        let da = a.finalize().unwrap();
        let db = b.finalize().unwrap();
        assert_eq!(da, db);
    }
}
