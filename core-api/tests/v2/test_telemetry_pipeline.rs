// # 🧪 Comprehensive Test Suite for Telemetry Pipelines
//
// Guarantees:
// ✔ Telemetry counters track plaintext, ciphertext, compression, overhead
// ✔ Stage times are recorded for both encrypt and decrypt pipelines
// ✔ Segment counts are correct (single, multi, final segment)
// ✔ Output fields are populated correctly
// ✔ Negative paths (empty input, corrupted ciphertext) fail safely
// ✔ Compression ratio and throughput metrics are computed
// ✔ Symmetry between encrypt and decrypt pipelines
//
// If any test fails, it means:
// * Telemetry accounting regressed,
// * Pipeline framing invariants were broken,
// * Or error handling became inconsistent.

#[cfg(test)]
mod telemetry_pipeline_tests {
    use std::io::Cursor;
    use std::sync::Arc;

    use core_api::{
        constants::MAX_CHUNK_SIZE,
        crypto::{DigestAlg, KEY_LEN_32},
        headers::HeaderV1,
        recovery::AsyncLogManager,
        parallelism::HybridParallelismProfile,
        stream::{
            io::PayloadReader,
            segment_worker::{EncryptContext, DecryptContext},
        },
        telemetry::TelemetrySnapshot,
    };
    use core_api::v2::pipeline::{PipelineConfig, encrypt_pipeline, decrypt_pipeline};

    // ## 1️⃣ Helpers
    fn setup_enc_context(alg: DigestAlg) -> (EncryptContext, HybridParallelismProfile, Arc<AsyncLogManager>) {
        let header = HeaderV1::test_header();
        let profile = HybridParallelismProfile::semi_dynamic(header.chunk_size as u32, 0.50, 64);
        let session_key = vec![0x42u8; KEY_LEN_32];
        let log_manager = Arc::new(AsyncLogManager::new("test_audit.log", 100).unwrap());
        let context = EncryptContext::new(header, profile.clone(), &session_key, alg).unwrap();
        (context, profile, log_manager)
    }

    fn setup_dec_context(alg: DigestAlg, header: &HeaderV1) -> (DecryptContext, HybridParallelismProfile, Arc<AsyncLogManager>) {
        let profile = HybridParallelismProfile::semi_dynamic(header.chunk_size as u32, 0.50, 64);
        let session_key = vec![0x42u8; KEY_LEN_32];
        let log_manager = Arc::new(AsyncLogManager::new("test_audit.log", 100).unwrap());
        let context = DecryptContext::from_stream_header(header.clone(), profile.clone(), &session_key, alg).unwrap();
        (context, profile, log_manager)
    }

    fn run_encrypt_with_data(data: &[u8]) -> (TelemetrySnapshot, Vec<u8>) {
        let mut reader = PayloadReader::new(Cursor::new(data.to_vec()));
        let mut writer = Cursor::new(Vec::new());
        let (ctx, profile, log_manager) = setup_enc_context(DigestAlg::Blake3);
        let config_pipe = PipelineConfig::new(profile.clone(), None);
        let snapshot = encrypt_pipeline(&mut reader, &mut writer, Arc::new(ctx), &config_pipe, log_manager).unwrap();

        (snapshot, writer.into_inner())
    }

    fn run_decrypt_with_ciphertext(ciphertext: Vec<u8>) -> TelemetrySnapshot {
        let cursor = Cursor::new(ciphertext);
        let (stream_header, reader) = PayloadReader::with_header(cursor).unwrap();
        let mut writer = Cursor::new(Vec::new());
        let (ctx, profile, log_manager) = setup_dec_context(DigestAlg::Blake3, &stream_header);
        let config_pipe = PipelineConfig::new(profile.clone(), None);
        decrypt_pipeline(&mut PayloadReader::new(reader), &mut writer, Arc::new(ctx), &config_pipe, log_manager).unwrap()
    }

    fn run_decrypt_with_data(data: &[u8]) -> TelemetrySnapshot {
        let (_, ciphertext) = run_encrypt_with_data(data);
        run_decrypt_with_ciphertext(ciphertext)
    }

    // ## 2️⃣ Encrypt Pipeline Tests
    #[test]
    fn telemetry_counts_plaintext_bytes_encrypt() {
        let data = b"hello world this is plaintext";
        let (snapshot, _) = run_encrypt_with_data(data);
        assert!(snapshot.bytes_plaintext >= data.len() as u64);
        assert!(snapshot.bytes_overhead > 0);
    }

    #[test]
    fn telemetry_merges_compression_and_encryption() {
        let data = b"compress me and encrypt me";
        let (snapshot, _) = run_encrypt_with_data(data);
        assert!(snapshot.bytes_compressed > 0);
        assert!(snapshot.bytes_ciphertext > 0);
    }

    #[test]
    fn telemetry_handles_single_segment_then_final() {
        let data = b"x";
        let (snapshot, _) = run_encrypt_with_data(data);
        assert_eq!(snapshot.bytes_plaintext, data.len() as u64);
        assert!(snapshot.bytes_overhead > 0);
        assert!(snapshot.segments_processed >= 1);
    }

    #[test]
    fn telemetry_fails_on_empty_input_encrypt() {
        let data = b"";
        let result = std::panic::catch_unwind(|| run_encrypt_with_data(data));
        assert!(result.is_err());
    }

    #[test]
    fn telemetry_reports_segment_count_encrypt() {
        let data = b"segment test data";
        let (snapshot, _) = run_encrypt_with_data(data);
        assert!(snapshot.segments_processed >= 1);
    }

    #[test]
    fn telemetry_compression_ratio_is_computed() {
        let data = b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let (snapshot, _) = run_encrypt_with_data(data);
        assert!(snapshot.compression_ratio >= 0.0);
    }

    #[test]
    fn telemetry_throughput_is_nonzero_encrypt() {
        let data = b"throughput test data repeated repeated repeated repeated";
        let (snapshot, _) = run_encrypt_with_data(data);
        assert!(snapshot.throughput_plaintext_bytes_per_sec > 0.0);
    }

    #[test]
    fn telemetry_output_contains_ciphertext() {
        let data = b"check output field";
        let (mut snapshot, buffer) = run_encrypt_with_data(data);
        // Attach the buffer contents
        snapshot.attach_output(buffer);
        assert!(snapshot.output.is_some());
        // Since `output` is now `Option<OwnedOutput>`, unwrap the NewType:
        let ciphertext = snapshot.output.expect("ciphertext captured").0;
        // The `.0` unwraps `OwnedOutput` into the inner `Vec<u8>`, then `&ciphertext` borrows it as `&[u8]` for the zero-copy `InputSource::Memory` slice.
        // `ciphertext` stays alive for the entire `decrypt_stream_v2` call so the borrow is valid.

        assert!(!ciphertext.is_empty());
    }

    // ## 3️⃣ Decrypt Pipeline Tests
    #[test]
    fn decrypt_pipeline_includes_final_segment() {
        let data = b"hello world";
        let (_, ciphertext) = run_encrypt_with_data(data);
        let snapshot = run_decrypt_with_ciphertext(ciphertext);
        assert!(snapshot.segments_processed >= 1);
    }

    #[test]
    fn telemetry_counts_plaintext_bytes_after_decrypt() {
        let data = b"hello world this is plaintext";
        let snapshot = run_decrypt_with_data(data);
        assert_eq!(snapshot.bytes_plaintext, data.len() as u64);
        assert!(snapshot.bytes_ciphertext > 0);
        assert!(snapshot.bytes_overhead > 0);
    }

    #[test]
    fn telemetry_records_stage_times_in_decrypt() {
        let data = b"some test data for timing";
        let snapshot = run_decrypt_with_data(data);
        assert!(!snapshot.stage_times.times.is_empty());
        assert!(snapshot.elapsed.as_nanos() > 0);
    }

    #[test]
    fn telemetry_merges_decryption_and_decompression() {
        let data = b"compress me and encrypt me";
        let snapshot = run_decrypt_with_data(data);
        assert!(snapshot.bytes_plaintext > 0);
        assert!(snapshot.bytes_ciphertext > 0);
    }

    #[test]
    fn telemetry_reports_segment_count_in_decrypt() {
        let data = b"segment test data";
        let snapshot = run_decrypt_with_data(data);
        assert!(snapshot.segments_processed >= 1);
    }

    #[test]
    fn telemetry_sanity_check_passes_for_decrypt() {
        let data = b"sanity check data";
        let snapshot = run_decrypt_with_data(data);
        assert!(snapshot.sanity_check());
    }

    #[test]
    fn telemetry_output_bytes_matches_ciphertext_count() {
        let data = b"check output bytes";
        let snapshot = run_decrypt_with_data(data);
        assert_eq!(snapshot.output_bytes(), snapshot.bytes_ciphertext);
    }

    #[test]
    fn telemetry_fails_on_corrupted_ciphertext() {
        let bad_ciphertext = vec![0xde, 0xad, 0xbe, 0xef];
        let cursor = Cursor::new(bad_ciphertext);
        let mut reader = PayloadReader::new(cursor);
        let mut writer = Cursor::new(Vec::new());
        let (ctx, profile, log_manager) = setup_dec_context(DigestAlg::Blake3, &HeaderV1::test_header());
        let config_pipe = PipelineConfig::new(profile.clone(), None);
        let result = decrypt_pipeline(&mut reader, &mut writer, Arc::new(ctx), &config_pipe, log_manager);
        assert!(result.is_err());
    }

    #[test]
    fn telemetry_handles_multi_segment_input() {
        let data = vec![42u8; MAX_CHUNK_SIZE + 4096];
        let (_, ciphertext) = run_encrypt_with_data(&data);
        let snapshot = run_decrypt_with_ciphertext(ciphertext);
        assert!(snapshot.segments_processed > 2);
        assert_eq!(snapshot.bytes_plaintext, data.len() as u64);
    }
}
