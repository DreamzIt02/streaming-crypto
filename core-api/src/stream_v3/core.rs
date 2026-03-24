
// ## 2️⃣ `core.rs` — stable public API

use std::sync::Arc;

use crate::{
    crypto::derive_session_key_32, headers::HeaderV1, parallelism::HybridParallelismProfile, recovery::AsyncLogManager, 
    stream_v2::{
        ApiConfig, DecryptParams, EncryptParams, MasterKey, 
        core::{validate_decrypt_params, validate_encrypt_params},
        io::{InputSource, OutputSink, open_output},
        pipeline::PipelineConfig, segment_worker::{DecryptContext, EncryptContext}
    }, 
    stream_v3::pipeline::{decrypt_pipeline, decrypt_read_header, encrypt_pipeline}, telemetry::TelemetrySnapshot, types::StreamError
};

fn setup_enc_context(master_key: &MasterKey, header: &HeaderV1, config: ApiConfig)
    -> Result<(EncryptContext, HybridParallelismProfile, Arc<AsyncLogManager>), StreamError> 
{
    let session_key = derive_session_key_32(&master_key, header).map_err(StreamError::Crypto)?;
    // FIXME: HybridParallelismProfile:: must respect user provided config.parallelism
    let profile = HybridParallelismProfile::from_stream_header(header.clone(), config.parallelism)?;
    let context = EncryptContext::new(header.clone(), profile.clone(), &session_key, config.alg.unwrap())
        .map_err(StreamError::SegmentWorker)?;
    let log_manager = Arc::new(AsyncLogManager::new("stream_v2_enc.log", 100)?);

    Ok((context, profile, log_manager))
}

fn setup_dec_context(master_key: &MasterKey, header: &HeaderV1, config: ApiConfig)
    -> Result<(DecryptContext, HybridParallelismProfile, Arc<AsyncLogManager>), StreamError> 
{
    let session_key = derive_session_key_32(&master_key, header).map_err(StreamError::Crypto)?;

    // FIXME: HybridParallelismProfile:: must respect user provided config.parallelism
    let profile = HybridParallelismProfile::from_stream_header(header.clone(), config.parallelism)?;
    let context = DecryptContext::from_stream_header(header.clone(), profile.clone(), &session_key, config.alg.unwrap())
        .map_err(StreamError::SegmentWorker)?;
    let log_manager = Arc::new(AsyncLogManager::new("stream_v2_dec.log", 100)?);

    Ok((context, profile, log_manager))
}

/// 🔐 Encrypt stream (v3)
pub fn encrypt_stream_v3(
    input: InputSource,          // 👈 pass directly
    output: OutputSink,
    master_key: &MasterKey,
    params: EncryptParams,
    config: ApiConfig,
) -> Result<TelemetrySnapshot, StreamError> {
    // Validate parameters
    validate_encrypt_params(&master_key, &params, None, None)?;

    // Normalize with defaults
    let final_config = config.with_defaults();

    // Open output only
    let (writer, maybe_buf) = open_output(output, final_config.with_buf)?;

    // Setup crypto context, parallelism profile, and logging
    let (crypto, profile, log_manager) =
        setup_enc_context(&master_key, &params.header, final_config)?;
    let config_pipe = PipelineConfig::new(profile, maybe_buf.clone());

    // Wrap crypto in Arc before passing into pipeline
    let crypto = Arc::new(crypto);

    // Call the new pipeline (v3) — input is passed directly
    let mut snapshot = encrypt_pipeline(
        input,
        writer,
        crypto,
        &config_pipe,
        log_manager,
    )?;

    // --- Telemetry buffer extraction for tests ---
    if let Some(ref arc_buf) = maybe_buf {
        let buf = arc_buf.lock().unwrap();
        snapshot.attach_output(buf.clone()); // clone Vec<u8> into snapshot.output
    }

    Ok(snapshot)
}

/// 🔓 Decrypt stream (v3)
pub fn decrypt_stream_v3(
    input: InputSource,          // 👈 pass raw InputSource
    output: OutputSink,
    master_key: &MasterKey,
    params: DecryptParams,
    config: ApiConfig,
) -> Result<TelemetrySnapshot, StreamError> {
    // Validate parameters
    validate_decrypt_params(&master_key, &params, None, None)?;

    // Normalize with defaults
    let final_config = config.with_defaults();

    // ---- Read stream header ----
    // Assert reader is positioned correctly
    let (header, input1) = decrypt_read_header(input)?;
    // Open output only (pipeline expects a writer)
    let (writer, maybe_buf) = open_output(output, final_config.with_buf)?;

    // Setup crypto context, parallelism profile, and logging
    let (crypto, profile, log_manager) = setup_dec_context(&master_key, &header, final_config)?;
    let config_pipe = PipelineConfig::new(profile, maybe_buf.clone());

    // Wrap crypto in Arc before passing into pipeline
    let crypto = Arc::new(crypto);

    // Call the new pipeline (v3) — pass InputSource directly
    let mut snapshot = decrypt_pipeline(
        input1,      // 👈 no PayloadReader, just raw InputSource
        writer,
        crypto,
        &config_pipe,
        log_manager,
    )?;

    // --- Telemetry buffer extraction for tests ---
    if let Some(ref arc_buf) = maybe_buf {
        let buf = arc_buf.lock().unwrap();
        snapshot.attach_output(buf.clone()); // clone Vec<u8> into snapshot.output
    }

    Ok(snapshot)
}
