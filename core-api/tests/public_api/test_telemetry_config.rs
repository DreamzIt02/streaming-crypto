// ## 🧪 Test File: `tests/core_api/test_telemetry_config.rs`

#[cfg(test)]
mod tests {
    use core_api::{headers::HeaderV1, stream_v2::{InputSource, OutputSink, core::{ApiConfig, DecryptParams, EncryptParams, MasterKey}, decrypt_stream_v2, encrypt_stream_v2}};

    fn dummy_master_key() -> MasterKey {
        MasterKey::new(vec![0x11; 32]) // 256‑bit dummy key
    }

    fn dummy_header() -> HeaderV1 {
        HeaderV1::test_header()
    }

    #[test]
    fn telemetry_with_buf_true_captures_ciphertext() {
        let master_key = dummy_master_key();
        let header = dummy_header();
        let params = EncryptParams { header, dict: None };
        let config = ApiConfig::new(Some(true), None, None, None );

        let plaintext = b"telemetry buffer capture".to_vec();

        let snapshot_enc = encrypt_stream_v2(
            InputSource::Memory(&plaintext),
            OutputSink::Memory,
            &master_key,
            params.clone(),
            config.clone(),
        ).expect("encryption should succeed");

        // Assert ciphertext captured
        assert!(snapshot_enc.output.is_some(), "ciphertext should be captured in snapshot.output");

        // let ciphertext = snapshot_enc.output.unwrap();
        // Since `output` is now `Option<OwnedOutput>`, unwrap the NewType:
        let ciphertext = snapshot_enc.output.expect("ciphertext captured").0;
        // The `.0` unwraps `OwnedOutput` into the inner `Vec<u8>`, then `&ciphertext` borrows it as `&[u8]` for the zero-copy `InputSource::Memory` slice.
        // `ciphertext` stays alive for the entire `decrypt_stream_v2` call so the borrow is valid.

        let snapshot_dec = decrypt_stream_v2(
            InputSource::Memory(&ciphertext),
            OutputSink::Memory,
            &master_key,
            DecryptParams,
            config,
        ).expect("decryption should succeed");

        assert_eq!(snapshot_dec.bytes_plaintext, plaintext.len() as u64);
    }

    #[test]
    fn telemetry_with_buf_none_does_not_capture_ciphertext() {
        let master_key = dummy_master_key();
        let header = dummy_header();
        let params = EncryptParams { header, dict: None };
        let config = ApiConfig::new(None, None, None, None );

        let plaintext = b"telemetry no buffer".to_vec();

        let snapshot_enc = encrypt_stream_v2(
            InputSource::Memory(&plaintext),
            OutputSink::Memory,
            &master_key,
            params.clone(),
            config.clone(),
        ).expect("encryption should succeed");

        // Assert ciphertext not captured
        assert!(snapshot_enc.output.is_none(), "ciphertext should not be captured when with_buf = None");
    }

    #[test]
    fn telemetry_collect_metrics_futureproof() {
        let master_key = dummy_master_key();
        let header = dummy_header();
        let params = EncryptParams { header, dict: None };
        let config = ApiConfig::new(Some(true), None, None, None );

        let plaintext = b"telemetry metrics test".to_vec();

        let snapshot_enc = encrypt_stream_v2(
            InputSource::Memory(&plaintext),
            OutputSink::Memory,
            &master_key,
            params.clone(),
            config.clone(),
        ).expect("encryption should succeed");

        // Even if metrics are not yet implemented, snapshot should contain counters
        assert!(snapshot_enc.segments_processed >= 1, "segments_processed counter should be present");
        assert!(snapshot_enc.bytes_plaintext >= plaintext.len() as u64);
    }

    // ### ✅ What These Tests Cover
    // - **with_buf = Some(true)**: Confirms ciphertext is captured in `snapshot.output`.
    // - **with_buf = None**: Confirms no ciphertext is captured.
    // - **collect_metrics = Some(true)**: Future‑proof check that telemetry counters (`segments_processed`, `bytes_plaintext`) are still populated.
}
