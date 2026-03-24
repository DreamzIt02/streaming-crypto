// # 📂 `src/stream_v3/test_core.rs`

#[cfg(test)]
mod tests {
    use core_api::{
        constants::{MAGIC_DICT, MIN_DICT_LEN}, 
        headers::HeaderV1, 
        stream_v2::{
            InputSource, OutputSink, 
            core::{ApiConfig, DecryptParams, EncryptParams, MasterKey, validate_decrypt_params, validate_dictionary, validate_encrypt_params}, 
        },
        stream_v3::{
            decrypt_stream_v3, encrypt_stream_v3, 
        }
    };

    fn dummy_master_key() -> MasterKey {
        MasterKey::new(vec![0x11u8; 32]) // valid 32-byte key
    }

    fn dummy_header() -> HeaderV1 {
        HeaderV1 {
            chunk_size: 64 * 1024,
            // fill in other required fields with defaults or dummy values
            ..HeaderV1::test_header()
        }
    }

    // --- Validation Tests ---

    #[test]
    fn validate_encrypt_params_with_valid_key_and_defaults() {
        let params = EncryptParams {
            header: dummy_header(),
            dict: None,
        };
        let result = validate_encrypt_params(&dummy_master_key(), &params, None, None);
        assert!(result.is_ok(), "Expected valid params to pass");
    }

    #[test]
    fn validate_encrypt_params_with_invalid_key_len() {
        let params = EncryptParams {
            header: dummy_header(),
            dict: None,
        };
        let bad_key = MasterKey::new(vec![0x22u8; 15]); // invalid length
        let result = validate_encrypt_params(&bad_key, &params, None, None);
        assert!(result.is_err(), "Expected invalid master key length error");
    }

    #[test]
    fn validate_decrypt_params_with_valid_key_and_defaults() {
        let params = DecryptParams;
        let result = validate_decrypt_params(&dummy_master_key(), &params, None, None);
        assert!(result.is_ok(), "Expected valid decrypt params to pass");
    }

    #[test]
    fn validate_dictionary_none_and_empty() {
        assert!(validate_dictionary(None).is_ok());
        assert!(validate_dictionary(Some(&[])).is_ok());
    }

    #[test]
    fn validate_dictionary_invalid_payload() {
        let bad_dict = vec![0x00u8; MIN_DICT_LEN];
        assert!(validate_dictionary(Some(&bad_dict)).is_err());
    }

    #[test]
    fn validate_dictionary_valid_payload() {
        let mut dict = MAGIC_DICT.to_vec();
        dict.resize(MIN_DICT_LEN, 0xAA);
        assert!(validate_dictionary(Some(&dict)).is_ok());
    }

    // --- Encrypt/Decrypt Pipeline Tests ---
    #[test]
    fn encrypt_and_decrypt_roundtrip_minimal() {
        let master_key = dummy_master_key();
        let header = dummy_header();
        let params = EncryptParams { header: header.clone(), dict: None };
        let config = ApiConfig::new(Some(true), None, None, None );

        let plaintext = vec![0x55u8; 1024];
        let input = InputSource::Memory(&plaintext);

        // Encrypt
        // let (cursor, _) = open_output_cursor(OutputSink::Memory).unwrap();
        let snapshot_enc = encrypt_stream_v3(
            input, 
            OutputSink::Memory, 
            &master_key, 
            params.clone(), 
            config.clone()
        )
        .expect("encryption should succeed");

        // Recover the buffer from the cursor the pipeline wrote into
        // let ciphertext = snapshot_enc.output.unwrap();
        // Since `output` is now `Option<OwnedOutput>`, unwrap the NewType:
        let ciphertext = snapshot_enc.output.expect("ciphertext captured").0;
        // The `.0` unwraps `OwnedOutput` into the inner `Vec<u8>`, then `&ciphertext` borrows it as `&[u8]` for the zero-copy `InputSource::Memory` slice.
        // `ciphertext` stays alive for the entire `decrypt_stream_v3` call so the borrow is valid.

        // Assert header is present
        println!("Ciphertext LEN: {}", ciphertext.len());
        assert!(ciphertext.len() >= HeaderV1::LEN, "Ciphertext missing stream header");
        eprintln!(
            "[TEST] Ciphertext length = {}, header prefix = {:?}",
            ciphertext.len(),
            &ciphertext[..HeaderV1::LEN]
        );

        // Decrypt
        let input_dec = InputSource::Memory(&ciphertext);
        let snapshot_dec = decrypt_stream_v3(
            input_dec, 
            OutputSink::Memory, 
            &master_key, 
            DecryptParams, 
            config
        )
        .expect("decryption should succeed");

        assert_eq!(snapshot_dec.bytes_plaintext, plaintext.len() as u64);
    }

    #[test]
    fn encrypt_and_decrypt_roundtrip() {
        let master_key = dummy_master_key();
        let header = dummy_header();
        let params = EncryptParams { header: header.clone(), dict: None };
        let config = ApiConfig::new(Some(true), None, None, None );

        let plaintext = vec![0x55u8; 1024]; // 1 KiB of data
        let input = InputSource::Memory(&plaintext);
        let output = OutputSink::Memory;

        // Encrypt
        let snapshot_enc = encrypt_stream_v3(input, output, &master_key, params.clone(), config.clone())
            .expect("encryption should succeed");

        assert!(snapshot_enc.bytes_plaintext >= 1024);

        // Since `output` is now `Option<OwnedOutput>`, unwrap the NewType:
        let ciphertext = snapshot_enc.output.expect("ciphertext captured").0;
        // The `.0` unwraps `OwnedOutput` into the inner `Vec<u8>`, then `&ciphertext` borrows it as `&[u8]` for the zero-copy `InputSource::Memory` slice.
        // `ciphertext` stays alive for the entire `decrypt_stream_v3` call so the borrow is valid.

        // Decrypt
        let input_dec = InputSource::Memory(&ciphertext);
        let output_dec = OutputSink::Memory;
        let snapshot_dec = decrypt_stream_v3(input_dec, output_dec, &master_key, DecryptParams, config)
            .expect("decryption should succeed");

        assert_eq!(snapshot_dec.bytes_plaintext, 1024);
    }

    #[test]
    fn encrypt_stream_with_invalid_key_should_fail() {
        let bad_key = MasterKey::new(vec![0x33u8; 15]); // invalid length
        let header = dummy_header();
        let params = EncryptParams { header, dict: None };
        let config = ApiConfig::new(Some(false), None, None, None );

        let plaintext = vec![0x44u8; 512];
        let input = InputSource::Memory(&plaintext);
        let output = OutputSink::Memory;

        let result = encrypt_stream_v3(input, output, &bad_key, params, config);
        assert!(result.is_err(), "Expected encryption to fail with invalid key");
    }

    #[test]
    fn decrypt_stream_with_invalid_key_should_fail() {
        let bad_key = MasterKey::new(vec![0x33u8; 16]); // invalid length
        let input = InputSource::Memory(&vec![0x99u8; 128]);
        let output = OutputSink::Memory;
        let config = ApiConfig::new(Some(false), None, None, None );

        let result = decrypt_stream_v3(input, output, &bad_key, DecryptParams, config);
        assert!(result.is_err(), "Expected decryption to fail with invalid key");
    }

    #[test]
    fn encrypt_decrypt_stream_matches_snapshot_buf() {
        let plaintext = vec![0xCD; 256 * 1024]; // 256 KB payload

        let master_key = dummy_master_key();
        let header = dummy_header();
        let enc_params = EncryptParams { header, dict: None };
        let dec_params = DecryptParams { };

        let config = ApiConfig::new(Some(true), None, None, None); // with_buf = true

        // Run encryption
        let reader = std::io::Cursor::new(plaintext.clone());
        let snapshot_enc = encrypt_stream_v3(
            InputSource::Reader(Box::new(reader)),
            OutputSink::Memory, // use buffer sink
            &master_key,
            enc_params,
            config.clone(),
        )
        .expect("encryption failed");

        // The snapshot now contains the encrypted output buffer
        // let encrypted_buf = snapshot_enc.output.clone().expect("missing output buffer");
        // Since `output` is now `Option<OwnedOutput>`, unwrap the NewType:
        let ciphertext = snapshot_enc.output.expect("ciphertext captured").0;
        // The `.0` unwraps `OwnedOutput` into the inner `Vec<u8>`, then `&ciphertext` borrows it as `&[u8]` for the zero-copy `InputSource::Memory` slice.
        // `ciphertext` stays alive for the entire `decrypt_stream_v3` call so the borrow is valid.

        // Run decryption on that buffer
        let reader = std::io::Cursor::new(ciphertext);
        let snapshot_dec = decrypt_stream_v3(
            InputSource::Reader(Box::new(reader)),
            OutputSink::Memory,
            &master_key,
            dec_params,
            config.clone(),
        )
        .expect("decryption failed");

        // The snapshot now contains the decrypted output buffer
        // let decrypted_buf = snapshot_dec.output.clone().expect("missing output buffer");
        // Since `output` is now `Option<OwnedOutput>`, unwrap the NewType:
        let ciphertext_dec = snapshot_dec.output.expect("ciphertext captured").0;
        // The `.0` unwraps `OwnedOutput` into the inner `Vec<u8>`, then `&ciphertext` borrows it as `&[u8]` for the zero-copy `InputSource::Memory` slice.
        // `ciphertext` stays alive for the entire `decrypt_stream_v3` call so the borrow is valid.

        // Compare decrypted buffer against original plaintext
        assert_eq!(ciphertext_dec, plaintext);

        // Also verify telemetry counters are consistent
        assert!(snapshot_enc.frames_data == snapshot_dec.frames_data);
        assert_eq!(snapshot_dec.frames_digest, 4);      // (256 / 64) KB each segment has one digest frame
        assert_eq!(snapshot_dec.frames_terminator, 4);  // (256 / 64) KB each segment has one terminator frame
    }

}
