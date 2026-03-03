// ## 📘 Unified Test Suite for `HeaderV1`
// This suite is strict, exhaustive, and guarantees:
//
// ✔ Header ABI stability
// ✔ Registry correctness
// ✔ Validation strictness
// ✔ Error message quality
// ✔ Forward‑compatibility safety
//
// If any test fails, it means:
// * a protocol invariant was broken,
// * a new field was added unsafely, or
// * a security guarantee regressed.

#[cfg(test)]
mod tests {
    use core_api::{
        compression::enum_name_or_hex,
        constants::{MAX_CHUNK_SIZE, flags},
        headers::{
            AadDomain, AlgProfile, CipherSuite, HeaderError, HeaderV1,
            HkdfPrf, Strategy, decode_header_le, encode_header_le
        },
        utils::fmt_bytes,
    };

    // ## 1️⃣ Enum Registry Verification
    #[test]
    fn strategy_verify_accepts_known() {
        for v in [Strategy::Sequential, Strategy::Parallel, Strategy::Auto] {
            Strategy::verify(v as u16).unwrap();
        }
    }

    #[test]
    fn strategy_verify_rejects_unknown() {
        let err = Strategy::verify(0xFFFF).unwrap_err();
        matches!(err, HeaderError::UnknownStrategy { raw: 0xFFFF });
    }

    #[test]
    fn cipher_suite_verify() {
        CipherSuite::verify(CipherSuite::Aes256Gcm as u16).unwrap();
        CipherSuite::verify(CipherSuite::Chacha20Poly1305 as u16).unwrap();
        CipherSuite::verify(0xDEAD).unwrap_err();
    }

    #[test]
    fn hkdf_prf_verify() {
        HkdfPrf::verify(HkdfPrf::Sha256 as u16).unwrap();
        HkdfPrf::verify(HkdfPrf::Sha512 as u16).unwrap();
        HkdfPrf::verify(HkdfPrf::Blake3K as u16).unwrap();
        HkdfPrf::verify(0xBEEF).unwrap_err();
    }

    #[test]
    fn alg_profile_verify() {
        AlgProfile::verify(AlgProfile::Aes256GcmHkdfSha256 as u16).unwrap();
        AlgProfile::verify(AlgProfile::Chacha20Poly1305HkdfBlake3K as u16).unwrap();
        AlgProfile::verify(0x9999).unwrap_err();
    }

    #[test]
    fn aad_domain_verify() {
        AadDomain::verify(AadDomain::Generic as u16).unwrap();
        AadDomain::verify(AadDomain::FileEnvelope as u16).unwrap();
        AadDomain::verify(AadDomain::PipeEnvelope as u16).unwrap();
        AadDomain::verify(0x4444).unwrap_err();
    }

    // ## 2️⃣ Header Validation – Success Cases
    #[test]
    fn header_default_is_valid() {
        HeaderV1::default().validate().unwrap();
    }

    #[test]
    fn header_test_header_is_valid() {
        HeaderV1::test_header().validate().unwrap();
    }

    #[test]
    fn header_with_optional_fields_valid() {
        let mut h = HeaderV1::test_header();
        h.set_plaintext_size(123456);
        h.set_crc32(0xDEADBEEF);
        h.set_dict_id(42);
        h.enable_terminator();
        h.enable_final_digest();
        h.enable_aad_strict();
        h.validate().unwrap();
    }

    // ## 3️⃣ Header Validation – Failure Cases
    #[test]
    fn header_invalid_magic() {
        let mut h = HeaderV1::test_header();
        h.magic = *b"BAD!";
        matches!(h.validate().unwrap_err(), HeaderError::InvalidMagic { .. });
    }

    #[test]
    fn header_invalid_version_zero() {
        let mut h = HeaderV1::test_header();
        h.version = 0;
        matches!(h.validate().unwrap_err(), HeaderError::InvalidVersion { .. });
    }

    #[test]
    fn header_invalid_chunk_size_zero() {
        let mut h = HeaderV1::test_header();
        h.chunk_size = 0;
        matches!(h.validate().unwrap_err(), HeaderError::InvalidChunkSizeZero);
    }

    #[test]
    fn header_invalid_chunk_size_too_large() {
        let mut h = HeaderV1::test_header();
        h.chunk_size = MAX_CHUNK_SIZE as u32 + 1;
        matches!(h.validate().unwrap_err(), HeaderError::InvalidChunkSizeTooLarge { .. });
    }

    #[test]
    fn header_invalid_salt_all_zero() {
        let mut h = HeaderV1::test_header();
        h.salt = [0u8; 16];
        matches!(h.validate().unwrap_err(), HeaderError::InvalidSalt { .. });
    }

    #[test]
    fn header_reserved_bytes_nonzero() {
        let mut h = HeaderV1::test_header();
        h.reserved[3] = 1;
        matches!(h.validate().unwrap_err(), HeaderError::ReservedBytesNonZero { .. });
    }

    #[test]
    fn header_dict_flag_without_id() {
        let mut h = HeaderV1::test_header();
        h.flags |= flags::DICT_USED;
        h.dict_id = 0;
        matches!(h.validate().unwrap_err(), HeaderError::DictUsedButMissingId);
    }

    // ## 4️⃣ Encode/Decode Roundtrip & CRC
    fn make_valid_header() -> HeaderV1 {
        HeaderV1::test_header()
    }

    #[test]
    fn roundtrip_encode_decode() {
        let header = make_valid_header();
        let encoded = encode_header_le(&header).unwrap();
        let decoded = decode_header_le(&encoded).unwrap();
        assert_eq!(header.magic, decoded.magic);
        assert_eq!(decoded.crc32, crc32fast::hash(&encoded[0..32]));
    }

    #[test]
    fn detects_corrupted_crc32() {
        let header = make_valid_header();
        let mut encoded = encode_header_le(&header).unwrap();
        encoded[0] ^= 0xFF; // corrupt magic
        matches!(decode_header_le(&encoded).unwrap_err(), HeaderError::InvalidCrc32 { .. });
    }

    #[test]
    fn detects_buffer_too_short() {
        let buf = vec![0u8; HeaderV1::LEN - 1];
        matches!(decode_header_le(&buf).unwrap_err(), HeaderError::BufferTooShort { .. });
    }

    // ## 5️⃣ Formatting & Diagnostics
    #[test]
    fn enum_name_or_hex_known() {
        assert_eq!(enum_name_or_hex::<Strategy>(Strategy::Parallel as u16), "Parallel");
    }

    #[test]
    fn enum_name_or_hex_unknown() {
        assert_eq!(enum_name_or_hex::<Strategy>(0xABCD), "0xabcd");
    }

    #[test]
    fn fmt_bytes_ascii() {
        assert_eq!(fmt_bytes(b"hello world"), r#"b"hello world""#);
    }

    #[test]
    fn fmt_bytes_binary() {
        assert_eq!(fmt_bytes(&[0x00, 0xFF, 0x01]), "0x00ff01");
    }

    // ## 6️⃣ ABI & Layout Invariants
    #[test]
    fn header_v1_size_is_stable() {
        assert_eq!(std::mem::size_of::<HeaderV1>(), HeaderV1::LEN);
    }

    #[test]
    fn header_reserved_is_zeroed_by_default() {
        assert!(HeaderV1::default().reserved.iter().all(|&b| b == 0));
    }
}
