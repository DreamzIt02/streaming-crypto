// ## 🧪 Test File: `tests/core_api/test_compression_cipher_variants.rs`

#[cfg(test)]
mod tests {
    use core_api::{compression::codec_ids, constants::{cipher_ids, prf_ids}, headers::HeaderV1, stream::{InputSource, OutputSink, core::{ MasterKey} }};
    use core_api::v2::{core::{ApiConfig, DecryptParams, EncryptParams}, decrypt_stream_v2, encrypt_stream_v2};

    fn dummy_master_key() -> MasterKey {
        MasterKey::new(vec![0x11; 32]) // 256‑bit dummy key
    }

    fn base_header() -> HeaderV1 {
        HeaderV1::test_header()
    }

    fn run_roundtrip_with_header(header: HeaderV1, plaintext: Vec<u8>) {
        let master_key = dummy_master_key();
        let params_enc  = EncryptParams { header, dict: None, master_key: master_key.clone() };
        let params_dec      = DecryptParams { master_key: master_key };
        let config = ApiConfig::new(Some(true), None, None, None );

        let snapshot_enc = encrypt_stream_v2(
            InputSource::Memory(&plaintext),
            OutputSink::Memory,
            params_enc,
            config.clone(),
        ).expect("encryption should succeed");

        // Since `output` is now `Option<OwnedOutput>`, unwrap the NewType:
        // let ciphertext = snapshot_enc.output.expect("ciphertext captured");
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
    fn compression_disabled_auto() {
        let mut header = base_header();
        header.compression = codec_ids::AUTO;
        run_roundtrip_with_header(header, b"no compression test".to_vec());
    }

    #[test]
    fn compression_enabled_zstd() {
        let mut header = base_header();
        header.compression = codec_ids::ZSTD;
        run_roundtrip_with_header(header, b"zstd compression test".to_vec());
    }

    #[test]
    fn compression_enabled_lz4() {
        let mut header = base_header();
        header.compression = codec_ids::LZ4;
        run_roundtrip_with_header(header, b"lz4 compression test".to_vec());
    }

    #[test]
    fn cipher_aes256_gcm() {
        let mut header = base_header();
        header.cipher = cipher_ids::AES256_GCM;
        run_roundtrip_with_header(header, b"aes256-gcm cipher test".to_vec());
    }

    #[test]
    fn cipher_chacha20_poly1305() {
        let mut header = base_header();
        header.cipher = cipher_ids::CHACHA20_POLY1305;
        run_roundtrip_with_header(header, b"chacha20-poly1305 cipher test".to_vec());
    }

    #[test]
    fn prf_sha256() {
        let mut header = base_header();
        header.hkdf_prf = prf_ids::SHA256;
        run_roundtrip_with_header(header, b"sha256 prf test".to_vec());
    }

    #[test]
    fn prf_blake3k() {
        let mut header = base_header();
        header.hkdf_prf = prf_ids::BLAKE3K;
        run_roundtrip_with_header(header, b"blake3k prf test".to_vec());
    }

    // ### ✅ What These Tests Cover
    // - **Compression**: Disabled (`AUTO`), enabled (`ZSTD`, `LZ4`).
    // - **Cipher suites**: AES‑256‑GCM vs ChaCha20‑Poly1305.
    // - **PRFs**: SHA‑256 vs BLAKE3K.
}
