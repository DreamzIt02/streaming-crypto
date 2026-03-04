// ## 📝 pyo3-api/src/lib.rs

use pyo3::prelude::*;
use pyo3::types::PyBytes;

pub mod io;
pub mod ffi;

pub use io::*;
pub use ffi::*;

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
#[pyfunction]
pub fn encrypt<'py>(py: Python<'py>, data: &Bound<'py, PyBytes>,) -> PyResult<Bound<'py, PyBytes>> {
    // ✅ Extract pure Rust slice while GIL is held
    let input: &[u8] = data.as_bytes();

    // ✅ Now closure only captures &[u8] (which is Send)
    let encrypted: Vec<u8> = py.allow_threads(|| { core_encrypt(input) });

    Ok(PyBytes::new_bound(py, &encrypted))
}

#[pymodule]
pub fn streaming_crypto(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {

    // Register public api
    m.add_function(wrap_pyfunction!(encrypt, m)?)?;

    // Register pyo3 modules
    ffi::register(py, m)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pyo3::types::PyBytes;

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
}
