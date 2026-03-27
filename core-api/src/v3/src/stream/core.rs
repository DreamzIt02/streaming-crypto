
// ## 2️⃣ `core.rs` — stable public API

use std::sync::Arc;

use crate::{
    constants::{DEFAULT_QUEUE_CAP, DEFAULT_WORKERS, MAGIC_DICT, MAX_DICT_LEN, MIN_DICT_LEN, QUEUE_CAPS, WORKERS_COUNT}, 
    core::MasterKey, 
    crypto::{DigestAlg, derive_session_key_32}, 
    headers::HeaderV1, 
    parallelism::{HybridParallelismProfile, ParallelismConfig}, 
    recovery::AsyncLogManager, 
    stream::{
        io::{InputSource, OutputSink, open_output},
        segment_worker::{DecryptContext, EncryptContext}
    }, 
    telemetry::TelemetrySnapshot, types::StreamError
};

use crate::v3::{pipeline::types::PipelineConfig, stream::pipeline::{decrypt_pipeline, decrypt_read_header, encrypt_pipeline}};

#[derive(Debug, Clone)]
pub struct EncryptParams<'a> {
    pub header      : HeaderV1,
    pub dict        : Option<&'a [u8]>,
    pub master_key  : MasterKey,
}
impl<'a> EncryptParams<'a> {
    pub fn validate(&self) -> Result<(), StreamError> {
        MasterKey::validate(&self.master_key)?;
        validate_dictionary(self.dict.as_deref())?;
        // If HeaderV1 has validation logic, we can enable it here:
        // self.header.validate_header()?;
        Ok(())
    }
}
#[derive(Debug, Clone)]
pub struct DecryptParams {
    pub master_key  : MasterKey,
}
impl DecryptParams {
    pub fn validate(&self) -> Result<(), StreamError> {
        MasterKey::validate(&self.master_key)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ApiConfig {
    /// Whether to capture the output buffer in memory.
    /// - `None` or `Some(false)` → no buffer capture (production default).
    /// - `Some(true)` → capture buffer for tests/benchmarks.
    pub with_buf: Option<bool>,

    /// Whether to collect detailed metrics during pipeline execution.
    /// Currently unused, reserved for future expansion.
    pub collect_metrics: Option<bool>,

    /// 
    /// Supported digest algorithms (extensible).
    pub alg: Option<DigestAlg>,

    /// 
    /// Parallelism configuration.
    pub parallelism: Option<ParallelismConfig>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            with_buf: Some(false),      // default: no buffer
            collect_metrics: Some(false), // default: no metrics
            alg: Some(DigestAlg::Blake3), // default: Blake3
            parallelism: Some(ParallelismConfig::default()),
        }
    }
}

impl ApiConfig {
    pub fn new(with_buf: Option<bool>, collect_metrics: Option<bool>, alg: Option<DigestAlg>, parallelism: Option<ParallelismConfig>) -> Self {
        Self {
            with_buf: with_buf.or(Some(false)),
            collect_metrics: collect_metrics.or(Some(false)),
            alg: alg.or(Some(DigestAlg::Blake3)),
            parallelism: Some(parallelism.unwrap_or_default()),
        }
    }
    /// Merge user-provided values with defaults 
    pub fn with_defaults(self) -> Self { 
        let defaults = ApiConfig::default(); 
        
        Self { 
            with_buf: self.with_buf.or(defaults.with_buf), 
            collect_metrics: self.collect_metrics.or(defaults.collect_metrics), 
            alg: self.alg.or(defaults.alg), 
            parallelism: self.parallelism.or(defaults.parallelism), 
        } 
    }
}

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
    params: EncryptParams,
    config: ApiConfig,
) -> Result<TelemetrySnapshot, StreamError> {
    // Validate parameters
    validate_encrypt_params(&params, None, None)?;

    // Normalize with defaults
    let final_config = config.with_defaults();

    // Open output only
    let (writer, maybe_buf) = open_output(output, final_config.with_buf)?;

    // Setup crypto context, parallelism profile, and logging
    let (crypto, profile, log_manager) =
        setup_enc_context(&params.master_key, &params.header, final_config)?;
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
    params: DecryptParams,
    config: ApiConfig,
) -> Result<TelemetrySnapshot, StreamError> {
    // Validate parameters
    validate_decrypt_params(&params, None, None)?;

    // Normalize with defaults
    let final_config = config.with_defaults();

    // ---- Read stream header ----
    // Assert reader is positioned correctly
    let (header, input1) = decrypt_read_header(input)?;
    // Open output only (pipeline expects a writer)
    let (writer, maybe_buf) = open_output(output, final_config.with_buf)?;

    // Setup crypto context, parallelism profile, and logging
    let (crypto, profile, log_manager) = setup_dec_context(&params.master_key, &header, final_config)?;
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


pub fn validate_encrypt_params(
    params: &EncryptParams,
    workers: Option<usize>,
    queue_cap: Option<usize>,

) -> Result<(), StreamError> {
    // --- Master key length ---
    MasterKey::validate(&params.master_key)?;

    // --- Resolve defaults ---
    let w  = workers.unwrap_or(DEFAULT_WORKERS);
    let q  = queue_cap.unwrap_or(DEFAULT_QUEUE_CAP);

    if !WORKERS_COUNT.contains(&w) {
        return Err(StreamError::Validation(format!(
            "invalid workers count: {w}, must be one of {:?}",
            WORKERS_COUNT
        )));
    }
    if !QUEUE_CAPS.contains(&q) {
        return Err(StreamError::Validation(format!(
            "invalid queue capacity: {q}, must be one of {:?}",
            QUEUE_CAPS
        )));
    }

    params.validate()?;
    Ok(())
}

pub fn validate_decrypt_params(
    params: &DecryptParams,
    workers: Option<usize>,
    queue_cap: Option<usize>,
) -> Result<(), StreamError> {
    // --- Master key length ---
    MasterKey::validate(&params.master_key)?;

    // --- Resolve defaults ---
    let w  = workers.unwrap_or(DEFAULT_WORKERS);
    let q  = queue_cap.unwrap_or(DEFAULT_QUEUE_CAP);

    if !WORKERS_COUNT.contains(&w) {
        return Err(StreamError::Validation(format!(
            "invalid workers count: {w}, must be one of {:?}",
            WORKERS_COUNT
        )));
    }
    if !QUEUE_CAPS.contains(&q) {
        return Err(StreamError::Validation(format!(
            "invalid queue capacity: {q}, must be one of {:?}",
            QUEUE_CAPS
        )));
    }

    params.validate()?;
    Ok(())
}

pub fn validate_dictionary(dict: Option<&[u8]>) -> Result<(), StreamError> {
    match dict {
        None => Ok(()), // no dictionary supplied
        Some(d) if d.is_empty() => Ok(()), // empty Vec also means "no dictionary"
        Some(d) => {
            // Non-empty dictionary must pass validation
            if !is_valid_dictionary(d) {
                Err(StreamError::Validation("invalid dictionary payload".into()))
            } else {
                Ok(())
            }
        }
    }
}

pub fn is_valid_dictionary(dict: &[u8]) -> bool {
    // Replace with the actual validation logic:
    // e.g. check header bytes, length constraints, codec id, etc.
    if dict.len() < MIN_DICT_LEN || dict.len() > MAX_DICT_LEN {
        return false;
    }

    // First 4 bytes to be a magic number
    let magic = MAGIC_DICT;
    dict.len() >= magic.len() && &dict[..magic.len()] == magic
}
