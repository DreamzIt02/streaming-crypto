#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::sync::Arc;

    use core_api::InputSource;
    use core_api::constants::DEFAULT_CHUNK_SIZE;
    use core_api::crypto::{DigestAlg, KEY_LEN_32};
    use core_api::headers::{HeaderV1, Strategy};
    use core_api::pipeline::PipelineConfig;
    use core_api::recovery::AsyncLogManager;
    use core_api::parallelism::{HybridParallelismProfile, ParallelismConfig};
    use core_api::stream_v2::{framing::FrameHeader, io::PayloadReader, segment_worker::{EncryptContext, DecryptContext}, segmenting::SegmentHeader};
    use core_api::stream_v3::pipeline::{decrypt_read_header, decrypt_pipeline, encrypt_pipeline};
    use core_api::telemetry::TelemetrySnapshot;
    use core_api::types::StreamError;

    fn setup_enc_context(alg: DigestAlg) -> (EncryptContext, Arc<AsyncLogManager>) {
        let header = HeaderV1::test_header();
        let profile = HybridParallelismProfile::semi_dynamic(header.chunk_size as u32, 0.50, 64);
        let session_key = vec![0x42u8; KEY_LEN_32];
        let log_manager = Arc::new(AsyncLogManager::new("test_audit.log", 100).unwrap());

        let context = EncryptContext::new(header, profile, &session_key, alg).unwrap();
        (context, log_manager)
    }

    fn setup_dec_context(alg: DigestAlg) -> (DecryptContext, Arc<AsyncLogManager>) {
        let header = HeaderV1::test_header();
        let profile = HybridParallelismProfile::semi_dynamic(header.chunk_size as u32, 0.50, 64);
        let session_key = vec![0x42u8; KEY_LEN_32];
        let log_manager = Arc::new(AsyncLogManager::new("test_audit.log", 100).unwrap());

        let context = DecryptContext::from_stream_header(header, profile, &session_key, alg).unwrap();
        (context, log_manager)
    }

    fn run_encrypt_decrypt(
        plaintext: &[u8],
        profile: HybridParallelismProfile,
    ) -> Result<(Vec<u8>, TelemetrySnapshot), StreamError> {
        let (crypto_enc, log_enc) = setup_enc_context(DigestAlg::Sha256);
        let (crypto_dec, log_dec) = setup_dec_context(DigestAlg::Sha256);

        // Attach a monitor for telemetry/error reporting
        let config_pipe = PipelineConfig::new(profile.clone(),  None);

        // --- Encrypt ---
        let mut encrypted = Vec::new();
        let crypto_enc = Arc::new(crypto_enc);

        let enc_snapshot = encrypt_pipeline(
            InputSource::Memory(&plaintext), // pass Vec<u8> directly
            Cursor::new(&mut encrypted),
            crypto_enc,
            &config_pipe,
            log_enc,
        )?;
       
        // --- Decrypt ---
        let mut decrypted = Vec::new();
        let dec_cursor = InputSource::Memory(&encrypted); // InputSource for decrypt_read_header
        let (_header, dec_reader) = decrypt_read_header(dec_cursor)?; // returns InputSource<'a>
        let crypto_dec = Arc::new(crypto_dec);

        decrypt_pipeline(
            dec_reader,
            Cursor::new(&mut decrypted),
            crypto_dec,
            &config_pipe,
            log_dec,
        )?;

        Ok((decrypted, enc_snapshot))
    }

    #[test]
    fn encrypt_pipeline_exact_multiple_chunk_size_only() {
        // Setup context
        let (crypto_enc, log_enc) = setup_enc_context(DigestAlg::Sha256);

        let config_para = ParallelismConfig::new(2, 2, 0.5, 4);
        let profile = HybridParallelismProfile::from_stream_header(
            crypto_enc.header,
            Some(config_para),
        ).expect("valid parallelism profile");

        // Attach a monitor for telemetry/error reporting
        let config_pipe = PipelineConfig::new(profile.clone(), None);

        // Match the header's chunk_size (e.g. 64 KiB)
        let chunk_size = crypto_enc.header.chunk_size as usize;
        let num_segments = 3;
        let data = vec![0x11u8; chunk_size * num_segments]; // exact multiple of header chunk_size

        let crypto_enc = Arc::new(crypto_enc);

        // Encrypt to produce ciphertext
        let mut encrypted = Vec::new();
        let snapshot = encrypt_pipeline(
            InputSource::Memory(&data.clone()), // pass Vec<u8> directly
            Cursor::new(&mut encrypted),
            crypto_enc,
            &config_pipe,
            log_enc,
        ).expect("encryption pipeline should finish");

        println!("✓ Encryption succeeded: segments processed {}", snapshot.segments_processed);
        
        // Assert we got some output and telemetry
        assert!(!encrypted.is_empty(), "encrypted stream should not be empty");
        assert!(snapshot.segments_processed >= num_segments as u64);
    }

    #[test]
    fn decrypt_pipeline_exact_multiple_chunk_size() {
        let (crypto_enc, log_enc) = setup_enc_context(DigestAlg::Sha256);
        let (crypto_dec, log_dec) = setup_dec_context(DigestAlg::Sha256);
        let config_para = ParallelismConfig::new(2, 2, 0.5, 4);
        let profile = HybridParallelismProfile::from_stream_header(crypto_enc.header, Some(config_para)).unwrap();
        let config_pipe = PipelineConfig::new(profile.clone(), None);

        let chunk_size = crypto_enc.header.chunk_size as usize;
        let num_segments = 2;
        let data = vec![0x22u8; chunk_size * num_segments];

        let mut encrypted = Vec::new();
        let crypto_enc = Arc::new(crypto_enc);
        encrypt_pipeline(
            InputSource::Memory(&data.clone()), 
            Cursor::new(&mut encrypted),
            crypto_enc, 
            &config_pipe, 
            log_enc
        ).unwrap();

        let dec_cursor = InputSource::Memory(&encrypted);
        let (_header, dec_reader) = decrypt_read_header(dec_cursor).unwrap();
        let crypto_dec = Arc::new(crypto_dec);

        let mut decrypted = Vec::new();
        let snapshot = decrypt_pipeline(dec_reader, Cursor::new(&mut decrypted), crypto_dec, &config_pipe, log_dec).unwrap();

        assert_eq!(decrypted.len(), data.len());
        assert!(snapshot.segments_processed >= num_segments as u64);
    }

    #[test]
    fn encrypt_decrypt_roundtrip_single_thread() {
        let data = b"hello secure streaming world";
        let (out, _) = run_encrypt_decrypt(data, HybridParallelismProfile::single_threaded()).unwrap();
        assert_eq!(out, data);
    }

    #[test]
    fn encrypt_decrypt_roundtrip_parallel() {
        let data = vec![0xAB; 64 * 1024];
        let config_para = ParallelismConfig::new(4, 4, 0.5, 8);
        let profile = HybridParallelismProfile::with_strategy(Strategy::Parallel, DEFAULT_CHUNK_SIZE as u32, Some(config_para)).unwrap();
        let (out, _) = run_encrypt_decrypt(&data, profile).unwrap();
        assert_eq!(out, data);
    }

    #[test]
    fn header_mismatch_is_detected() {
        let (crypto_enc, log_enc) = setup_enc_context(DigestAlg::Sha256);
        let profile = HybridParallelismProfile::single_threaded();
        let config_pipe = PipelineConfig::new(profile.clone(), None);

        let data = b"attack at dawn";
        let mut encrypted = Vec::new();
        let crypto_enc = Arc::new(crypto_enc);
        encrypt_pipeline(InputSource::Memory(&data.clone()), Cursor::new(&mut encrypted), crypto_enc, &config_pipe, log_enc).unwrap();

        encrypted[22] ^= 0xFF; // corrupt header
        let dec_cursor = Cursor::new(encrypted);
        let err = PayloadReader::with_header(dec_cursor).unwrap_err();
        matches!(err, StreamError::Header(_));
    }

    #[test]
    fn detects_corrupted_stream() {
        let (crypto_enc, log_enc) = setup_enc_context(DigestAlg::Sha256);
        let (crypto_dec, log_dec) = setup_dec_context(DigestAlg::Sha256);
        let profile = HybridParallelismProfile::single_threaded();
        let config_pipe = PipelineConfig::new(profile.clone(), None);

        let data = b"this will be corrupted";
        let mut encrypted = Vec::new();
        let crypto_enc = Arc::new(crypto_enc);
        encrypt_pipeline(InputSource::Memory(&data.clone()), Cursor::new(&mut encrypted), crypto_enc, &config_pipe, log_enc).unwrap();

        let ct_start = HeaderV1::LEN + SegmentHeader::LEN + FrameHeader::LEN + 5;
        encrypted[ct_start] ^= 0xAA;

        let dec_cursor = InputSource::Memory(&encrypted);
        let (_header, dec_reader) = decrypt_read_header(dec_cursor).unwrap();
        let crypto_dec = Arc::new(crypto_dec);

        let err = decrypt_pipeline(dec_reader, Cursor::new(Vec::new()), crypto_dec, &config_pipe, log_dec).unwrap_err();
        matches!(err, StreamError::Validation(_));
    }

    #[test]
    fn encrypt_pipeline_exact_multiple_chunk_size() {
        // Setup context
        let (crypto_enc, log_enc) = setup_enc_context(DigestAlg::Sha256);

        let config_para = ParallelismConfig::new(2, 2, 0.5, 4);
        let profile = HybridParallelismProfile::from_stream_header(
            crypto_enc.header,
            Some(config_para),
        ).expect("valid parallelism profile");

        // Attach a monitor for telemetry/error reporting
        let config_pipe = PipelineConfig::new(profile.clone(), None);

        // Match the header's chunk_size (e.g. 64 KiB)
        let chunk_size = crypto_enc.header.chunk_size as usize;
        let num_segments = 3;
        let data = vec![0x11u8; chunk_size * num_segments]; // exact multiple of header chunk_size

        let enc_reader = InputSource::Memory(&data.clone());
        let crypto_enc = Arc::new(crypto_enc);

        // Encrypt to produce ciphertext
        let mut encrypted = Vec::new();
        let snapshot = encrypt_pipeline(
            enc_reader,
            Cursor::new(&mut encrypted),
            crypto_enc,
            &config_pipe,
            log_enc,
        ).expect("encryption pipeline should finish");

        println!("✓ Encryption succeeded: segments processed {}", snapshot.segments_processed);

        // Assert we got some output and telemetry
        assert!(!encrypted.is_empty(), "encrypted stream should not be empty");
        assert!(snapshot.segments_processed >= num_segments as u64);
    }

    #[test]
    fn exact_multiple_of_chunk_size_final_segment() {
        let chunk_size = DEFAULT_CHUNK_SIZE;
        let num_segments = 5;
        let data = vec![0xABu8; chunk_size * num_segments];
        let config_para = ParallelismConfig::new(2, 2, 0.5, 4);
        let profile = HybridParallelismProfile::with_strategy(Strategy::Parallel, DEFAULT_CHUNK_SIZE as u32, Some(config_para)).unwrap();
        let (decrypted, enc_snapshot) = run_encrypt_decrypt(&data, profile).unwrap();
        assert_eq!(decrypted, data);
        assert!(enc_snapshot.segments_processed >= num_segments as u64);
    }

    #[test]
    fn preserves_order_under_parallelism() {
        let mut data = Vec::new();
        for i in 0..10_000u32 {
            data.extend_from_slice(&i.to_le_bytes());
        }

        let config_para = ParallelismConfig::new(6, 6, 0.5, 12);
        let profile = HybridParallelismProfile::with_strategy(
            Strategy::Parallel,
            DEFAULT_CHUNK_SIZE as u32,
            Some(config_para),
        ).unwrap();

        let (out, _) = run_encrypt_decrypt(&data, profile).unwrap();
        assert_eq!(out, data);
    }

    #[test]
    fn exact_chunk_boundary() {
        let chunk = DEFAULT_CHUNK_SIZE;
        let data = vec![1u8; chunk * 10];

        let config_para = ParallelismConfig::new(2, 2, 0.5, 4);
        let profile = HybridParallelismProfile::with_strategy(
            Strategy::Parallel,
            chunk as u32,
            Some(config_para),
        ).unwrap();

        let (out, _) = run_encrypt_decrypt(&data, profile).unwrap();
        assert_eq!(out, data);
    }

    #[test]
    fn empty_input_produces_error() {
        let data = [];

        let profile = HybridParallelismProfile::single_threaded();
        // let config_pipe = PipelineConfig::new(profile.clone(), None);

        let result = run_encrypt_decrypt(&data, profile);
        assert!(result.is_err());
    }

    #[test]
    fn bounded_backpressure_does_not_deadlock() {
        let chunk = DEFAULT_CHUNK_SIZE;
        let data = vec![42u8; 32 * 1024 * 1024]; // large payload

        let config_para = ParallelismConfig::new(4, 4, 0.5, 1);
        let profile = HybridParallelismProfile::with_strategy(
            Strategy::Parallel,
            chunk as u32,
            Some(config_para),
        ).unwrap();

        let (out, _) = run_encrypt_decrypt(&data, profile).unwrap();
        assert_eq!(out, data);
    }
}
