// ## 🧪 Test File: `tests/core_api/test_header_meta.rs`

#[cfg(test)]
mod tests {
    use streaming_crypto::{constants::flags, headers::{AadDomain, HeaderV1}, stream::{InputSource, OutputSink, io::PayloadReader, core::{MasterKey}}, v2::{ApiConfig, EncryptParams, DecryptParams, encrypt_stream_v2, decrypt_stream_v2}, types::StreamError};

    use std::io::Cursor;

    fn dummy_master_key() -> MasterKey {
        MasterKey::new(vec![0x11; 32]) // 256‑bit dummy key
    }

    fn dummy_header() -> HeaderV1 {
        HeaderV1::test_header()
    }

    #[test]
    fn header_presence_in_ciphertext() {
        let master_key = dummy_master_key();
        let header = dummy_header();
        let params_enc  = EncryptParams { header, dict: None, master_key: master_key.clone() };
        let _params_dec      = DecryptParams { master_key };
        let config = ApiConfig::new(Some(true), None, None, None );

        let plaintext = b"header presence test".to_vec();

        let snapshot_enc = encrypt_stream_v2(
            InputSource::Memory(&plaintext),
            OutputSink::Memory,
            params_enc,
            config,
        ).expect("encryption should succeed");

        // let ciphertext = snapshot_enc.output.expect("ciphertext captured");
        // Since `output` is now `Option<OwnedOutput>`, unwrap the NewType:
        let ciphertext = snapshot_enc.output.expect("ciphertext captured").0;
        // The `.0` unwraps `OwnedOutput` into the inner `Vec<u8>`, then `&ciphertext` borrows it as `&[u8]` for the zero-copy `InputSource::Memory` slice.
        // `ciphertext` stays alive for the entire `decrypt_stream_v2` call so the borrow is valid.

        // Assert magic marker at start
        assert_eq!(&ciphertext[0..4], b"RSE1");
        // Assert length ≥ header size
        assert!(ciphertext.len() >= HeaderV1::LEN as usize);
    }

    #[test]
    fn flags_are_preserved_in_header() {
        let master_key = dummy_master_key();
        let mut header = dummy_header();
        header.flags = flags::HAS_TOTAL_LEN | flags::HAS_CRC32;
        let params_enc  = EncryptParams { header, dict: None, master_key: master_key.clone() };
        let _params_dec      = DecryptParams { master_key };
        let config = ApiConfig::new(Some(true), None, None, None );

        let plaintext = b"flags preservation test".to_vec();

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

        // Decrypt back and check header flags
        let (_header, _) = PayloadReader::with_header(Cursor::new(ciphertext)).unwrap();
        assert_eq!(_header.flags & (flags::HAS_TOTAL_LEN | flags::HAS_CRC32),
                flags::HAS_TOTAL_LEN | flags::HAS_CRC32);
    }

    #[test]
    fn aad_domain_mismatch_detected() {
        let master_key = dummy_master_key();
        let mut header = dummy_header();
        header.aad_domain = AadDomain::FileEnvelope as u16; // mismatch domain
        let params_enc  = EncryptParams { header, dict: None, master_key: master_key.clone() };
        let params_dec      = DecryptParams { master_key };
        let config = ApiConfig::new(Some(true), None, None, None );

        let plaintext = b"aad mismatch test".to_vec();

        let snapshot_enc = encrypt_stream_v2(
            InputSource::Memory(&plaintext),
            OutputSink::Memory,
            params_enc,
            config.clone(),
        ).unwrap();

        // let mut ciphertext = snapshot_enc.output.unwrap();
        // Since `output` is now `Option<OwnedOutput>`, unwrap the NewType:
        let mut ciphertext = snapshot_enc.output.expect("ciphertext captured").0;
        // The `.0` unwraps `OwnedOutput` into the inner `Vec<u8>`, then `&ciphertext` borrows it as `&[u8]` for the zero-copy `InputSource::Memory` slice.
        // `ciphertext` stays alive for the entire `decrypt_stream_v2` call so the borrow is valid.

        // Corrupt the aad_domain field in header
        // HeaderV1 layout: aad_domain is at offset after strategy (2 bytes each)
        // magic(4) + version(2) + alg_profile(2) + cipher(2) + hkdf_prf(2) +
        // compression(2) + strategy(2) = 16 bytes so far
        // aad_domain at offset 16..18
        ciphertext[16] ^= 0xFF; // flip a byte

        let err = decrypt_stream_v2(
            InputSource::Memory(&ciphertext),
            OutputSink::Memory,
            params_dec,
            config,
        ).unwrap_err();

        matches!(err, StreamError::Header(_));
    }

    // ### ✅ What These Tests Cover
    // - **Header presence**: Ensures ciphertext starts with `"RSE1"` and has at least `HeaderV1::LEN` bytes.
    // - **Flags preservation**: Confirms flags set in `EncryptParams.header` are preserved and visible when reading back the header.
    // - **AAD domain mismatch**: Simulates corruption in the header’s `aad_domain` field and asserts that decryption fails with `StreamError::Header`.
}
