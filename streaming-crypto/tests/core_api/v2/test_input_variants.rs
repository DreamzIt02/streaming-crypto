// ## 🧪 Test File: `tests/core_api/test_input_variants.rs`

#[cfg(test)]
mod tests {
    use streaming_crypto::{headers::HeaderV1, stream::{InputSource, OutputSink, core::{MasterKey}}, v2::{ApiConfig, EncryptParams, DecryptParams, encrypt_stream_v2, decrypt_stream_v2}};

    use std::fs;
    use std::io::Cursor;
    use tempfile::NamedTempFile;

    fn dummy_master_key() -> MasterKey {
        MasterKey::new(vec![0x11; 32]) // 256‑bit dummy key
    }

    fn dummy_header() -> HeaderV1 {
        HeaderV1::test_header()
    }

    #[test]
    fn roundtrip_memory_input() {
        let master_key = dummy_master_key();
        let header = dummy_header();
        let params = EncryptParams { header, dict: None };
        let config = ApiConfig::new(Some(true), None, None, None );

        let plaintext = b"hello from memory".to_vec();

        let snapshot_enc = encrypt_stream_v2(
            InputSource::Memory(&plaintext),
            OutputSink::Memory,
            &master_key,
            params.clone(),
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
            &master_key,
            DecryptParams,
            config,
        ).expect("decryption should succeed");

        assert_eq!(snapshot_dec.bytes_plaintext, plaintext.len() as u64);
    }

    #[test]
    fn roundtrip_file_input() {
        let master_key = dummy_master_key();
        let header = dummy_header();
        let params = EncryptParams { header, dict: None };
        let config = ApiConfig::new(Some(true), None, None, None );

        let plaintext = b"hello from file".to_vec();

        // Write plaintext to a temp file
        let tmpfile = NamedTempFile::new().unwrap();
        fs::write(tmpfile.path(), &plaintext).unwrap();

        // Encrypt from file input
        let snapshot_enc = encrypt_stream_v2(
            InputSource::File(tmpfile.path().to_path_buf()),
            OutputSink::Memory,
            &master_key,
            params.clone(),
            config.clone(),
        ).unwrap();

        // let ciphertext = snapshot_enc.output.unwrap();
        // Since `output` is now `Option<OwnedOutput>`, unwrap the NewType:
        let ciphertext = snapshot_enc.output.expect("ciphertext captured").0;
        // The `.0` unwraps `OwnedOutput` into the inner `Vec<u8>`, then `&ciphertext` borrows it as `&[u8]` for the zero-copy `InputSource::Memory` slice.
        // `ciphertext` stays alive for the entire `decrypt_stream_v2` call so the borrow is valid.

        // Decrypt back
        let snapshot_dec = decrypt_stream_v2(
            InputSource::Memory(&ciphertext),
            OutputSink::Memory,
            &master_key,
            DecryptParams,
            config,
        ).unwrap();

        assert_eq!(snapshot_dec.bytes_plaintext, plaintext.len() as u64);
    }

    #[test]
    fn roundtrip_reader_input() {
        let master_key = dummy_master_key();
        let header = dummy_header();
        let params = EncryptParams { header, dict: None };
        let config = ApiConfig::new(Some(true), None, None, None );

        let plaintext = b"hello from reader".to_vec();
        let cursor = Cursor::new(plaintext.clone());

        let snapshot_enc = encrypt_stream_v2(
            InputSource::Reader(Box::new(cursor)),
            OutputSink::Memory,
            &master_key,
            params.clone(),
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
            &master_key,
            DecryptParams,
            config,
        ).unwrap();

        assert_eq!(snapshot_dec.bytes_plaintext, plaintext.len() as u64);
    }
}
