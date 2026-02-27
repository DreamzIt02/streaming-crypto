// ## 📝 streaming-crypto/src/lib.rs

/// Encrypts data by XORing each byte with 0xAA.
///
/// # Examples
///
/// ```
/// use core_api::encrypt;
///
/// let data = vec![1, 2, 3];
/// let encrypted = encrypt(&data);
/// assert_eq!(encrypted[0], 1 ^ 0xAA);
#[cfg(feature = "core-api")]
pub use core_api::encrypt; // re-export the FFI wrapper

pub mod ffi;
pub mod py;

#[cfg(feature = "ffi-api")]
pub use ffi::*;
#[cfg(feature = "pyo3-api")]
pub use py::*;
