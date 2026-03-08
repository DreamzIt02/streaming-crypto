// ## 📝 pyo3-api/src/lib.rs
#![allow(unexpected_cfgs)]

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use std::cell::Cell;

pub mod io;
pub mod api;
pub mod constants;
pub mod headers;
pub mod frames;
pub mod segments;
pub mod telemetry;
pub mod errors;
pub mod parallelism;
pub mod crypto;

pub use io::*;
pub use api::*;
pub use constants::*;
pub use headers::*;
pub use telemetry::*;
pub use errors::*;
pub use parallelism::*;
pub use crypto::*;


thread_local! {
    pub static INPUT_COPIES: Cell<usize> = Cell::new(0);
    pub static OUTPUT_COPIES: Cell<usize> = Cell::new(0);
}

// Helper accessors used by ffi_api and telemetry
pub fn increment_input_copies() {
    INPUT_COPIES.with(|c| c.set(c.get() + 1));
}
pub fn increment_output_copies() {
    OUTPUT_COPIES.with(|c| c.set(c.get() + 1));
}
pub fn get_input_copies() -> usize {
    INPUT_COPIES.with(|c| c.get())
}
pub fn get_output_copies() -> usize {
    OUTPUT_COPIES.with(|c| c.get())
}
pub fn reset_copy_counters() {
    INPUT_COPIES.with(|c| c.set(0));
    OUTPUT_COPIES.with(|c| c.set(0));
}

// Import the core Rust implementation from core-api
use core_api::encrypt as core_encrypt;

/// # Examples
///
/// ```rust,no_run
/// use pyo3::prelude::*;
/// use pyo3_api::encrypt;
/// use pyo3::types::PyBytes;
///
/// Python::with_gil(|py| {
///     let data = PyBytes::new_bound(py, &[1, 2, 3]);
///     
///     let encrypted = encrypt(py, &data).unwrap();
/// 
///     assert_eq!(encrypted[0], 1 ^ 0xAA);
///     assert_eq!(encrypted[1], 2 ^ 0xAA);
///     assert_eq!(encrypted[2], 3 ^ 0xAA);
/// });
/// ```
#[pyfunction(name = "encrypt")]
pub fn encrypt<'py>(py: Python<'py>, data: &Bound<'py, PyBytes>,) -> PyResult<Bound<'py, PyBytes>> {
    // ✅ Extract pure Rust slice while GIL is held
    let input: &[u8] = data.as_bytes();

    // ✅ Now closure only captures &[u8] (which is Send)
    let encrypted: Vec<u8> = py.allow_threads(|| { core_encrypt(input) });

    Ok(PyBytes::new_bound(py, &encrypted))
}

// #[pymodule(name = "streaming_crypto")]
// pub fn streaming_crypto(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {

//     // Export from the root module (streaming_crypto/***submodule***/__init__.py)
//     // Register constants
//     let s = PyModule::new_bound(py, "constants")?;
//     let _ = constants::register_constants(py, &s)?;
//     m.add_submodule(&s)?;

//     // Register headers
//     let s = PyModule::new_bound(py, "headers")?;
//     let _ = headers::register_headers(py, &s)?;
//     m.add_submodule(&s)?;

//     // Register segments
//     let s = PyModule::new_bound(py, "segments")?;
//     let _ = segments::register_segments(py, &s)?;
//     m.add_submodule(&s)?;

//     // Register frames
//     let s = PyModule::new_bound(py, "frames")?;
//     let _ = frames::register_frames(py, &s)?;
//     m.add_submodule(&s)?;

//     // Register crypto
//     let s = PyModule::new_bound(py, "crypto")?;
//     let _ = crypto::register_crypto(py, &s)?;
//     m.add_submodule(&s)?;

//     // Register parallelism
//     let s = PyModule::new_bound(py, "parallelism")?;
//     let _ = parallelism::register_parallelism(py, &s)?;
//     m.add_submodule(&s)?;

//     // Register pyo3 io
//     let s = PyModule::new_bound(py, "io")?;
//     let _ = io::register_io(py, &s)?;
//     m.add_submodule(&s)?;

//     // Export from the root module (streaming_crypto/__init__.py)
//     // Register public api
//     m.add_function(wrap_pyfunction!(encrypt, m)?)?;

//     // Register errors
//     let _ = errors::register_errors(py, m);
    
//     // Register telemetry
//     let _ = telemetry::register_telemetry(py, m);

//     // Register pyo3 api
//     let _ = ffi::register_api(py, m)?;

//     Ok(())
// }

#[pymodule(name = "streaming_crypto")]
pub fn streaming_crypto(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    let sys_modules = py.import_bound("sys")?
        .getattr("modules")?;

    // Register constants
    let s = PyModule::new_bound(py, "constants")?;
    constants::register_constants(py, &s)?;
    m.add_submodule(&s)?;
    sys_modules.set_item("streaming_crypto.constants", &s)?;

    // Register headers
    let s = PyModule::new_bound(py, "headers")?;
    headers::register_headers(py, &s)?;
    m.add_submodule(&s)?;
    sys_modules.set_item("streaming_crypto.headers", &s)?;

    // Register segments
    let s = PyModule::new_bound(py, "segments")?;
    segments::register_segments(py, &s)?;
    m.add_submodule(&s)?;
    sys_modules.set_item("streaming_crypto.segments", &s)?;

    // Register frames
    let s = PyModule::new_bound(py, "frames")?;
    frames::register_frames(py, &s)?;
    m.add_submodule(&s)?;
    sys_modules.set_item("streaming_crypto.frames", &s)?;

    // Register crypto
    let s = PyModule::new_bound(py, "crypto")?;
    crypto::register_crypto(py, &s)?;
    m.add_submodule(&s)?;
    sys_modules.set_item("streaming_crypto.crypto", &s)?;

    // Register parallelism
    let s = PyModule::new_bound(py, "parallelism")?;
    parallelism::register_parallelism(py, &s)?;
    m.add_submodule(&s)?;
    sys_modules.set_item("streaming_crypto.parallelism", &s)?;

    // Register io
    let s = PyModule::new_bound(py, "io")?;
    io::register_io(py, &s)?;
    m.add_submodule(&s)?;
    sys_modules.set_item("streaming_crypto.io", &s)?;

    // Register public api
    m.add_function(wrap_pyfunction!(encrypt, m)?)?;

    // Register errors
    errors::register_errors(py, m)?;

    // Register telemetry
    telemetry::register_telemetry(py, m)?;

    // Register pyo3 api
    api::register_api(py, m)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::api::{PyApiConfig, PyDecryptParams, PyEncryptParams, py_decrypt_stream_v2, py_encrypt_stream_v2};

    use super::*;
    use core_api::constants::{HEADER_V1, MAGIC_RSE1};
    use pyo3::types::{PyBytes};

    #[test]
    fn test_encrypt_py_api() {
        Python::with_gil(|py| {
            let data = PyBytes::new_bound(py, &[1, 2, 3]);

            // Pass raw slice
            let encrypted = encrypt(py, &data).unwrap();

            assert_eq!(encrypted[0], 1 ^ 0xAA);
            assert_eq!(encrypted[1], 2 ^ 0xAA);
            assert_eq!(encrypted[2], 3 ^ 0xAA);
        });
    }

    fn make_header(_py: Python) -> PyHeaderV1 {
        PyHeaderV1 {
            alg_profile:  PyAlgProfile::Chacha20Poly1305HkdfBlake3K,
            cipher:       PyCipherSuite::Chacha20Poly1305,
            hkdf_prf:     PyHkdfPrf::Blake3K,
            compression:  PyCompressionCodec::Auto,
            strategy:     PyStrategy::Auto,
            aad_domain:   PyAadDomain::Generic,
            magic:        MAGIC_RSE1,
            version:      HEADER_V1,
            flags:        0,
            chunk_size:   64 * 1024,   // 64 KiB
            plaintext_size: 0,
            crc32:        0,
            dict_id:      0,
            salt:         [1u8; 16],
            key_id:       0,
            parallel_hint: 1,
            enc_time_ns:  0,
            reserved:     [0u8; 8],
        }
    }

    fn make_params_enc(py: Python) -> PyEncryptParams {
        PyEncryptParams {
            header:     make_header(py),
            dict:       None,
            master_key: vec![0u8; 32],  // 32-byte zeroed key
        }
    }
    fn make_params_dec(_py: Python) -> PyDecryptParams {
        PyDecryptParams {
            master_key: vec![0u8; 32],  // 32-byte zeroed key
        }
    }

    fn make_config() -> PyApiConfig {
        PyApiConfig::new(
            Some(true),   // with_buf: capture output into TelemetrySnapshot.output
            Some(true),   // collect_metrics
            None,
            None,
        )
    }

    // ── Main test ────────────────────────────────────────────────────────────
    #[test]
    fn test_encrypt_stream_v2_zero_copy_input_one_copy_output() {
        Python::with_gil(|py| {
            reset_copy_counters();  // safe — thread-local, only affects this thread

            // ── Input: PyBytes (immutable) → zero-copy path ─────────────────
            let plaintext = vec![0x42u8; 1024];  // 1 KiB
            let py_input  = PyBytes::new_bound(py, &plaintext).into_py(py);
            let py_output = PyBytes::new_bound(py, &[]).into_py(py); // Memory sink

            let params = make_params_enc(py);
            let config = make_config();

            // ── Call ─────────────────────────────────────────────────────────
            let snapshot = py_encrypt_stream_v2(
                py,
                py_input,
                py_output,
                params,
                config,
            ).expect("encryption should succeed");

            // ── Assertions ───────────────────────────────────────────────────

            // Output was captured
            let output: &Py<PyBytes> = snapshot
                .output
                .as_ref()
                .expect("output should be Some for Memory sink");

            let output_len = output.bind(py).len().unwrap();
            assert!(
                output_len > plaintext.len(),
                "ciphertext ({} bytes) should be larger than plaintext ({} bytes)",
                output_len,
                plaintext.len(),
            );

            // Telemetry sanity
            assert_eq!(
                snapshot.bytes_plaintext as usize,
                plaintext.len(),
                "bytes_plaintext should match input size"
            );
            assert!(
                snapshot.throughput_plaintext_bytes_per_sec > 0.0,
                "throughput should be non-zero"
            );
            assert!(
                snapshot.elapsed_sec > 0.0,
                "elapsed should be non-zero"
            );
            assert!(
                snapshot.segments_processed > 0,
                "at least one segment should be processed"
            );

            // ── Zero-copy input assertion ────────────────────────────────────
            // PyBytes path must NOT have incremented INPUT_COPIES.
            // If our pipeline ever copies the input slice, this will catch it.
            // Assert DELTA — immune to parallel test interference
            assert_eq!(get_input_copies(),  0, "PyBytes input must be zero-copy");

            // ── One-copy output assertion ────────────────────────────────────
            // Exactly one copy is expected: Vec<u8> → PyBytes (Python heap).
            // If OUTPUT_COPIES > 1, an intermediate buffer was introduced.
            assert_eq!(get_output_copies(), 1, "Memory output must be exactly one copy");

            println!("[zero-copy test] plaintext={} bytes, ciphertext={} bytes, elapsed={:.6}s, throughput={:.2} B/s",
                snapshot.bytes_plaintext,
                output_len,
                snapshot.elapsed_sec,
                snapshot.throughput_plaintext_bytes_per_sec,
            );
        });
    }

    // ── Roundtrip: encrypt → decrypt ─────────────────────────────────────────
    #[test]
    fn test_encrypt_decrypt_roundtrip_memory() {
        Python::with_gil(|py| {
            reset_copy_counters();  // safe — thread-local, only affects this thread

            let plaintext = vec![0x42u8; 1024];
            // Encrypt
            let py_input  = PyBytes::new_bound(py, &plaintext).into_py(py);
            let py_output = PyBytes::new_bound(py, &[]).into_py(py);

            let enc_snapshot = py_encrypt_stream_v2(
                py,
                py_input,
                py_output,
                make_params_enc(py),
                make_config(),
            ).expect("encryption should succeed");

            let ciphertext_obj = enc_snapshot.output
                .as_ref()
                .expect("ciphertext should be captured");

            // Decrypt — feed PyBytes ciphertext back in (zero-copy input)
            let py_cipher_input = ciphertext_obj.clone_ref(py).into_py(py);
            let py_dec_output   = PyBytes::new_bound(py, &[]).into_py(py);

            let dec_snapshot = py_decrypt_stream_v2(
                py,
                py_cipher_input,
                py_dec_output,
                make_params_dec(py),
                make_config(),
            ).expect("decryption should succeed");

            let recovered = dec_snapshot.output
                .as_ref()
                .expect("decrypted output should be captured");

            // Roundtrip = 2 output copies: one for encrypt, one for decrypt
            assert_eq!(get_input_copies(),  0, "PyBytes input must be zero-copy");
            assert_eq!(get_output_copies(), 2, "roundtrip must be exactly two copies (enc + dec)");

            let recovered_bytes = recovered.bind(py).as_bytes();
            assert_eq!(
                recovered_bytes, plaintext.as_slice(),
                "decrypted output must match original plaintext"
            );
        });
    }
}
