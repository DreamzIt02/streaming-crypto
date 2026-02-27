// ## 📝 streaming-crypto/src/py.rs

/// # Examples
///
/// ```
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
#[cfg(feature = "pyo3-api")]
pub use pyo3_api::encrypt; // re-export the PyO3 wrapper

#[cfg(feature = "pyo3-api")]
pub use pyo3_api::streaming_crypto; // re-export the #[pymodule]