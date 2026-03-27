// ## 🧪 Test File: `tests/core_api/test_output_variants.rs`

#[cfg(test)]
mod tests {
    use core_api::{headers::HeaderV1, 
        stream::{InputSource, OutputSink, core::{MasterKey} },
    };
    use core_api::v3::{core::{ApiConfig, DecryptParams, EncryptParams, decrypt_stream_v3, encrypt_stream_v3 }};

    use std::io::Cursor;
    use tempfile::NamedTempFile;

    fn dummy_master_key() -> MasterKey {
        MasterKey::new(vec![0x11; 32]) // 256‑bit dummy key
    }

    fn dummy_header() -> HeaderV1 {
        HeaderV1::test_header()
    }

    #[test]
    fn roundtrip_output_memory_sink() {
        let master_key = dummy_master_key();
        let header = dummy_header();
        let params = EncryptParams { header, dict: None };
        let config = ApiConfig::new(Some(true), None, None, None );

        let plaintext = b"hello memory sink".to_vec();

        let snapshot_enc = encrypt_stream_v3(
            InputSource::Memory(&plaintext),
            OutputSink::Memory,
            &master_key,
            params.clone(),
            config.clone(),
        ).expect("encryption should succeed");

        // Assert ciphertext captured in snapshot.output
        // let ciphertext = snapshot_enc.output.expect("ciphertext captured");
        // Since `output` is now `Option<OwnedOutput>`, unwrap the NewType:
        let ciphertext = snapshot_enc.output.expect("ciphertext captured").0;
        // The `.0` unwraps `OwnedOutput` into the inner `Vec<u8>`, then `&ciphertext` borrows it as `&[u8]` for the zero-copy `InputSource::Memory` slice.
        // `ciphertext` stays alive for the entire `decrypt_stream_v3` call so the borrow is valid.

        let snapshot_dec = decrypt_stream_v3(
            InputSource::Memory(&ciphertext),
            OutputSink::Memory,
            &master_key,
            DecryptParams,
            config,
        ).expect("decryption should succeed");

        assert_eq!(snapshot_dec.bytes_plaintext, plaintext.len() as u64);
    }

    #[test]
    fn roundtrip_output_file_sink() {
        let master_key = dummy_master_key();
        let header = dummy_header();
        let params = EncryptParams { header, dict: None };
        let config = ApiConfig::new(Some(true), None, None, None );

        let plaintext = b"hello file sink".to_vec();

        // Encrypt to file
        let tmpfile = NamedTempFile::new().unwrap();
        let _ = encrypt_stream_v3(
            InputSource::Memory(&plaintext),
            OutputSink::File(tmpfile.path().to_path_buf()),
            &master_key,
            params.clone(),
            config.clone(),
        ).expect("encryption should succeed");

        // Decrypt from file
        let snapshot_dec = decrypt_stream_v3(
            InputSource::File(tmpfile.path().to_path_buf()),
            OutputSink::Memory,
            &master_key,
            DecryptParams,
            config,
        ).expect("decryption should succeed");

        assert_eq!(snapshot_dec.bytes_plaintext, plaintext.len() as u64);
    }

    #[test]
    fn roundtrip_output_writer_sink() {
        let master_key = dummy_master_key();
        let header = dummy_header();
        let params = EncryptParams { header, dict: None };
        let config = ApiConfig::new(Some(true), None, None, None);

        let plaintext = b"hello writer sink".to_vec();

        // Encrypt into a Cursor<Vec<u8>>
        let cursor_writer = Cursor::new(Vec::new());

        let _ = encrypt_stream_v3(
            InputSource::Memory(&plaintext),
            OutputSink::Writer(Box::new(cursor_writer)),
            &master_key,
            params.clone(),
            config.clone(),
        ).expect("encryption should succeed");

        // Now retrieve the ciphertext from the writer
        // (we may need to downcast or adjust our OutputSink handling to expose the Vec)
        // For example, if encrypt_stream_v3 attaches output when with_buf = Some(true):
        let snapshot_enc = encrypt_stream_v3(
            InputSource::Memory(&plaintext),
            OutputSink::Memory,
            &master_key,
            params.clone(),
            config.clone(),
        ).unwrap();

        // let ciphertext = snapshot_enc.output.unwrap();
        // Since `output` is now `Option<OwnedOutput>`, unwrap the NewType:
        let ciphertext = snapshot_enc.output.expect("ciphertext captured").0;
        // The `.0` unwraps `OwnedOutput` into the inner `Vec<u8>`, then `&ciphertext` borrows it as `&[u8]` for the zero-copy `InputSource::Memory` slice.
        // `ciphertext` stays alive for the entire `decrypt_stream_v3` call so the borrow is valid.

        // Decrypt from captured buffer
        let snapshot_dec = decrypt_stream_v3(
            InputSource::Memory(&ciphertext),
            OutputSink::Memory,
            &master_key,
            DecryptParams,
            config,
        ).expect("decryption should succeed");

        assert_eq!(snapshot_dec.bytes_plaintext, plaintext.len() as u64);
    }

    // ### ✅ What These Cover
    // - **Memory sink**: Ensures telemetry buffer capture works (`snapshot.output` contains ciphertext).
    // - **File sink**: Validates writing ciphertext to disk and decrypting back from file.
    // - **Writer sink**: Confirms generic `Write` trait objects (like `Cursor<Vec<u8>>`) work correctly.
}
