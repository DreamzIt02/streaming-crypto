// ## 📝 pyo3-api/src/lib.rs
#![allow(unexpected_cfgs)]

use pyo3::prelude::*;
use pyo3::types::PyBytes;

pub mod io;
pub mod ffi;
pub mod constants;
pub mod headers;
pub mod frames;
pub mod segments;
pub mod telemetry;
pub mod errors;
pub mod parallelism;
pub mod crypto;

pub use io::*;
pub use ffi::*;
pub use constants::*;
pub use headers::*;
pub use telemetry::*;
pub use errors::*;
pub use parallelism::*;
pub use crypto::*;

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

#[pymodule(name = "streaming_crypto")]
pub fn streaming_crypto(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {

    // Register public api
    m.add_function(wrap_pyfunction!(encrypt, m)?)?;

    // Register constants
    let _ = constants::register_constants(py, m);

    // Register errors
    let _ = errors::register_errors(py, m);

    // Register headers
    let _ = headers::register_headers(py, m);

    // Register segments
    let _ = segments::register_segments(py, m);

    // Register frames
    let _ = frames::register_frames(py, m);

    // Register crypto
    let _ = crypto::register_crypto(py, m);

    // Register telemetry
    let _ = telemetry::register_telemetry(py, m);

    // Register parallelism
    let _ = parallelism::register_parallelism(py, m);

    // Register pyo3 io
    let _ = io::register_io(py, m)?;

    // Register pyo3 api
    let _ = ffi::register_api(py, m)?;

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
