// ## 🧪 Test File: `tests/pyo3_api/test_telemetry_config.rs`
#[cfg(feature = "pyo3-api")]
#[cfg(test)]
mod tests {
    use pyo3::prelude::*;
    use pyo3::Py;
    use pyo3::{Python, IntoPy, types::{PyBytes}};

    use core_api::constants::{MAGIC_RSE1, HEADER_V1};

    use streaming_crypto::{PyHeaderV1, PyCompressionCodec, PyAlgProfile, PyCipherSuite, PyHkdfPrf, PyStrategy, PyAadDomain};
    use streaming_crypto::{reset_copy_counters, get_output_copies, get_input_copies};
    use streaming_crypto::v3::{PyApiConfig, PyDecryptParams, PyEncryptParams, py_encrypt_stream_v3};

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
    fn _make_params_dec(_py: Python) -> PyDecryptParams {
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
    fn test_encrypt_stream_v3_zero_copy_input_one_copy_output() {
        Python::with_gil(|py| {
            reset_copy_counters();  // safe — thread-local, only affects this thread

            // ── Input: PyBytes (immutable) → zero-copy path ─────────────────
            let plaintext = vec![0x42u8; 1024];  // 1 KiB
            let py_input  = PyBytes::new_bound(py, &plaintext).into_py(py);
            let py_output = PyBytes::new_bound(py, &[]).into_py(py); // Memory sink

            let params = make_params_enc(py);
            let config = make_config();

            // ── Call ─────────────────────────────────────────────────────────
            let snapshot = py_encrypt_stream_v3(
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
    
}
