// ## 🧪 Test File: `tests/core_api/test_public_api.rs`

#[cfg(test)]
mod tests {
    use streaming_crypto::{compression::CompressionCodec, headers::{AadDomain, AlgProfile, CipherSuite, HeaderV1, HkdfPrf, Strategy}, stream::{InputSource, OutputSink, core::{MasterKey}}, v3::{ApiConfig, EncryptParams, DecryptParams, encrypt_stream_v3, decrypt_stream_v3}, types::StreamError};

    fn dummy_master_key() -> MasterKey {
        MasterKey::new(vec![0x11; 32]) // 256‑bit dummy key
    }

    fn dummy_header() -> HeaderV1 {
        HeaderV1 {
            magic: *b"RSE1",
            version: 1,
            alg_profile: AlgProfile::Aes256GcmHkdfSha256 as u16,
            cipher: CipherSuite::Chacha20Poly1305 as u16,
            hkdf_prf: HkdfPrf::Sha256 as u16,
            compression: CompressionCodec::Auto as u16,
            strategy: Strategy::Auto as u16,
            aad_domain: AadDomain::Generic as u16,
            flags: 0,
            chunk_size: 64 * 1024,
            plaintext_size: 0,
            crc32: 0,
            dict_id: 0,
            salt: [1u8; 16],
            key_id: 0,
            parallel_hint: 0,
            enc_time_ns: 0,
            reserved: [0; 8],
        }
    }
    
    #[test]
    fn roundtrip_memory_output() {
        let master_key = dummy_master_key();
        let header = dummy_header();
        let params_enc  = EncryptParams { header, dict: None, master_key: master_key.clone() };
        let params_dec      = DecryptParams { master_key };
        let config = ApiConfig::new(Some(true), None, None, None );

        let plaintext = b"hello world".to_vec();

        let snapshot_enc = encrypt_stream_v3(
            InputSource::Memory(&plaintext),
            OutputSink::Memory,
            params_enc,
            config.clone(),
        ).expect("encryption should succeed");

        // let ciphertext = snapshot_enc.output.expect("ciphertext should be captured");
        // Since `output` is now `Option<OwnedOutput>`, unwrap the NewType:
        let ciphertext = snapshot_enc.output.expect("ciphertext captured").0;
        // The `.0` unwraps `OwnedOutput` into the inner `Vec<u8>`, then `&ciphertext` borrows it as `&[u8]` for the zero-copy `InputSource::Memory` slice.
        // `ciphertext` stays alive for the entire `decrypt_stream_v3` call so the borrow is valid.

        let snapshot_dec = decrypt_stream_v3(
            InputSource::Memory(&ciphertext),
            OutputSink::Memory,
            params_dec,
            config,
        ).expect("decryption should succeed");

        assert_eq!(snapshot_dec.bytes_plaintext, plaintext.len() as u64);
    }

    #[test]
    fn detects_corrupted_ciphertext() {
        let master_key = dummy_master_key();
        let header = dummy_header();
        let params_enc  = EncryptParams { header, dict: None, master_key: master_key.clone() };
        let params_dec      = DecryptParams { master_key };
        let config = ApiConfig::new(Some(true), None, None, None );

        let plaintext = vec![0xAB; 1024];
        let snapshot_enc = encrypt_stream_v3(
            InputSource::Memory(&plaintext),
            OutputSink::Memory,
            params_enc,
            config.clone(),
        ).unwrap();

        // let mut ciphertext = snapshot_enc.output.unwrap();
        // Since `output` is now `Option<OwnedOutput>`, unwrap the NewType:
        let mut ciphertext = snapshot_enc.output.expect("ciphertext captured").0;
        // The `.0` unwraps `OwnedOutput` into the inner `Vec<u8>`, then `&ciphertext` borrows it as `&[u8]` for the zero-copy `InputSource::Memory` slice.
        // `ciphertext` stays alive for the entire `decrypt_stream_v3` call so the borrow is valid.

        ciphertext[HeaderV1::LEN + 10] ^= 0xFF; // flip a byte

        let err = decrypt_stream_v3(
            InputSource::Memory(&ciphertext),
            OutputSink::Memory,
            params_dec,
            config,
        ).unwrap_err();

        matches!(err, StreamError::Validation(_));
    }
}