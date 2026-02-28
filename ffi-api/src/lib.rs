// ## 📝 ffi-api/src/lib.rs

// Import the core Rust implementation
use core_api::encrypt as core_encrypt;

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
/// ```
/// use std::slice;
/// use ffi_api::encrypt;
///
/// let data = vec![1, 2, 3];
/// let ptr = encrypt(data.as_ptr(), data.len());
/// let encrypted = unsafe { slice::from_raw_parts(ptr, data.len()) };
/// assert_eq!(encrypted[0], 1 ^ 0xAA);
/// ```
#[no_mangle]
pub extern "C" fn encrypt(data: *const u8, len: usize) -> *mut u8 {
    let slice = unsafe { std::slice::from_raw_parts(data, len) };
    let encrypted: Vec<u8> = core_encrypt(slice);
    let mut boxed = encrypted.into_boxed_slice();
    let ptr = boxed.as_mut_ptr();
    std::mem::forget(boxed);
    ptr
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::slice;

    #[test]
    fn test_encrypt_ffi_api() {
        let data = vec![1, 2, 3];
        let ptr = encrypt(data.as_ptr(), data.len());

        // Reconstruct slice from raw pointer
        let encrypted = unsafe { slice::from_raw_parts(ptr, data.len()) };

        assert_eq!(encrypted[0], 1 ^ 0xAA);
        assert_eq!(encrypted[1], 2 ^ 0xAA);
        assert_eq!(encrypted[2], 3 ^ 0xAA);
    }
}
