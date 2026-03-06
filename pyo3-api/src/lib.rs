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
    ffi::register_api(py, m)?;

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
