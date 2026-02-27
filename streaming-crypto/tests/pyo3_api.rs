#[cfg(feature = "pyo3-api")]
#[test]
fn test_encrypt_py_wrapper() {
    use pyo3::Python;
    use pyo3::types::PyBytes;

    Python::with_gil(|py| {
        let data = PyBytes::new_bound(py, &[1, 2, 3]);

        // Pass raw slice
        let encrypted = streaming_crypto::encrypt(py, &data).unwrap();

        assert_eq!(encrypted[0], 1 ^ 0xAA);
        assert_eq!(encrypted[1], 2 ^ 0xAA);
        assert_eq!(encrypted[2], 3 ^ 0xAA);
    });
}