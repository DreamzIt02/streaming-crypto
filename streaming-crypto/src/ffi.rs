// ## 📝 streaming-crypto/src/ffi.rs

/// FFI wrapper for encryption.
///
/// # Safety
/// Returns a raw pointer. Caller must manage memory.

/// FFI wrapper for encryption.
///
/// # Safety
/// This function returns a raw pointer. The caller must manage memory.
///
/// # Examples
///
/// ```
/// use std::slice;
/// use ffi_api::encrypt;
///
/// let data = vec![1, 2, 3];
/// let ptr = encrypt(data.as_ptr(), data.len());
/// let encrypted = unsafe { slice::from_raw_parts(ptr, data.len()) };
/// assert_eq!(encrypted[0], 1 ^ 0xAA);
#[cfg(feature = "ffi-api")]
pub use ffi_api::encrypt; // re-export the FFI wrapper