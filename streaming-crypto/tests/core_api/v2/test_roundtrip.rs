// ## 🧪 Test File: `tests/core_api/test_roundtrip.rs`
#[cfg(test)]
mod tests {
    use streaming_crypto::{headers::HeaderV1, stream::{InputSource, OutputSink, core::{MasterKey}}, v2::{ApiConfig, EncryptParams, DecryptParams, encrypt_stream_v2, decrypt_stream_v2}, types::StreamError};

    fn dummy_master_key() -> MasterKey {
        MasterKey::new(vec![0x11; 32]) // 256‑bit dummy key
    }

    fn dummy_header() -> HeaderV1 {
        HeaderV1::test_header()
    }

    #[test]
    fn roundtrip_minimal_plaintext() {
        let master_key = dummy_master_key();
        let header = dummy_header();
        let params_enc  = EncryptParams { header, dict: None, master_key: master_key.clone() };
        let params_dec      = DecryptParams { master_key };
        let config = ApiConfig::new(Some(true), None, None, None );

        let plaintext = b"hello world".to_vec();

        let snapshot_enc = encrypt_stream_v2(
            InputSource::Memory(&plaintext),
            OutputSink::Memory,
            params_enc,
            config.clone(),
        ).expect("encryption should succeed");

        // let ciphertext = snapshot_enc.output.expect("ciphertext captured");
        // Since `output` is now `Option<OwnedOutput>`, unwrap the NewType:
        let ciphertext = snapshot_enc.output.expect("ciphertext captured").0;
        // The `.0` unwraps `OwnedOutput` into the inner `Vec<u8>`, then `&ciphertext` borrows it as `&[u8]` for the zero-copy `InputSource::Memory` slice.
        // `ciphertext` stays alive for the entire `decrypt_stream_v2` call so the borrow is valid.

        let snapshot_dec = decrypt_stream_v2(
            InputSource::Memory(&ciphertext),
            OutputSink::Memory,
            params_dec,
            config,
        ).expect("decryption should succeed");

        assert_eq!(snapshot_dec.bytes_plaintext, plaintext.len() as u64);
    }

    #[test]
    fn roundtrip_large_plaintext() {
        let master_key = dummy_master_key();
        let header = dummy_header();
        let params_enc  = EncryptParams { header, dict: None, master_key: master_key.clone() };
        let params_dec      = DecryptParams { master_key };
        let config = ApiConfig::new(Some(true), None, None, None );

        let plaintext = vec![0xAB; 8 * 1024 * 1024]; // 8 MB

        let snapshot_enc = encrypt_stream_v2(
            InputSource::Memory(&plaintext),
            OutputSink::Memory,
            params_enc,
            config.clone(),
        ).unwrap();

        // let ciphertext = snapshot_enc.output.unwrap();
        // Since `output` is now `Option<OwnedOutput>`, unwrap the NewType:
        let ciphertext = snapshot_enc.output.expect("ciphertext captured").0;
        // The `.0` unwraps `OwnedOutput` into the inner `Vec<u8>`, then `&ciphertext` borrows it as `&[u8]` for the zero-copy `InputSource::Memory` slice.
        // `ciphertext` stays alive for the entire `decrypt_stream_v2` call so the borrow is valid.

        let snapshot_dec = decrypt_stream_v2(
            InputSource::Memory(&ciphertext),
            OutputSink::Memory,
            params_dec,
            config,
        ).unwrap();

        assert_eq!(snapshot_dec.bytes_plaintext, plaintext.len() as u64);
    }

    #[test]
    fn roundtrip_exact_chunk_boundaries() {
        let master_key = dummy_master_key();
        let mut header = dummy_header();
        header.chunk_size = 1024; // small chunk size for test
        let params_enc  = EncryptParams { header, dict: None, master_key: master_key.clone() };
        let params_dec      = DecryptParams { master_key };
        let config = ApiConfig::new(Some(true), None, None, None );

        let plaintext = vec![0x22; header.chunk_size as usize * 3]; // exactly 3 chunks

        let snapshot_enc = encrypt_stream_v2(
            InputSource::Memory(&plaintext),
            OutputSink::Memory,
            params_enc,
            config.clone(),
        ).unwrap();

        // let ciphertext = snapshot_enc.output.unwrap();
        // Since `output` is now `Option<OwnedOutput>`, unwrap the NewType:
        let ciphertext = snapshot_enc.output.expect("ciphertext captured").0;
        // The `.0` unwraps `OwnedOutput` into the inner `Vec<u8>`, then `&ciphertext` borrows it as `&[u8]` for the zero-copy `InputSource::Memory` slice.
        // `ciphertext` stays alive for the entire `decrypt_stream_v2` call so the borrow is valid.

        let snapshot_dec = decrypt_stream_v2(
            InputSource::Memory(&ciphertext),
            OutputSink::Memory,
            params_dec,
            config,
        ).unwrap();

        assert_eq!(snapshot_dec.bytes_plaintext, plaintext.len() as u64);
    }

    #[test]
    fn roundtrip_empty_input_errors() {
        let master_key = dummy_master_key();
        let header = dummy_header();
        let params_enc  = EncryptParams { header, dict: None, master_key: master_key.clone() };
        let _params_dec      = DecryptParams { master_key };
        let config = ApiConfig::new(Some(true), None, None, None );

        let plaintext: Vec<u8> = vec![];

        let err = encrypt_stream_v2(
            InputSource::Memory(&plaintext),
            OutputSink::Memory,
            params_enc,
            config,
        ).unwrap_err();

        matches!(err, StreamError::Validation(_));
    }
}
