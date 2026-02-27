#[cfg(feature = "core-api")]
#[test]
fn test_encrypt_core_wrapper() {
    let data = vec![1, 2, 3];
    let encrypted = streaming_crypto::encrypt(&data);
    assert_eq!(encrypted.len(), 3);
    // sanity check: XOR with 0xAA
    assert_eq!(encrypted[0], 1 ^ 0xAA);
}