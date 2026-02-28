#[cfg(feature = "ffi-api")]
#[test]
fn test_encrypt_ffi_wrapper() {
    use std::slice;
    use ffi_api::encrypt;

    let data = vec![1, 2, 3];
    let ptr = encrypt(data.as_ptr(), data.len());

    // reconstruct slice from raw pointer
    let encrypted = unsafe { slice::from_raw_parts(ptr, data.len()) };

    assert_eq!(encrypted[0], 1 ^ 0xAA);
    assert_eq!(encrypted[1], 2 ^ 0xAA);
    assert_eq!(encrypted[2], 3 ^ 0xAA);
}