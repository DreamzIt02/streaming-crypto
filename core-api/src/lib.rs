// ## 📝 core-api/src/lib.rs

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
/// ```
pub fn encrypt(data: &[u8]) -> Vec<u8> {
    data.iter().map(|b| b ^ 0xAA).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_core_api() {
        let data = vec![1, 2, 3];
        let encrypted = encrypt(&data);

        // Check length matches
        assert_eq!(encrypted.len(), data.len());

        // Check XOR transformation
        assert_eq!(encrypted[0], 1 ^ 0xAA);
        assert_eq!(encrypted[1], 2 ^ 0xAA);
        assert_eq!(encrypted[2], 3 ^ 0xAA);
    }
}
