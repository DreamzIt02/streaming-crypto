// ## 🧪 Test File: `tests/pyo3_api/test_roundtrip.rs`
#[cfg(feature = "pyo3-api")]
#[cfg(test)]
mod tests {
    use pyo3::{Python, IntoPy, types::{PyBytes, PyBytesMethods}};

    use core_api::constants::{MAGIC_RSE1, HEADER_V1};

    use streaming_crypto::{PyHeaderV1, PyCompressionCodec, PyAlgProfile, PyCipherSuite, PyHkdfPrf, PyStrategy, PyAadDomain};
    use streaming_crypto::{reset_copy_counters, get_output_copies, get_input_copies};
    use streaming_crypto::v2::{PyApiConfig, PyDecryptParams, PyEncryptParams, py_encrypt_stream_v2, py_decrypt_stream_v2};

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
