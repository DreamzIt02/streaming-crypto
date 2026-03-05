// ## 📝 pyo3-api/src/headers.rs

use core_api::{compression::codec_ids, constants::{aad_domain_ids, alg_profile_ids, cipher_ids, prf_ids, strategy_ids}, headers::HeaderV1};
use pyo3::prelude::*;
use num_enum::TryFromPrimitive;

/// Compression codec identifiers
#[pyclass(name = "CompressionCodec", eq, eq_int)]
#[repr(u16)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, TryFromPrimitive)]
pub enum PyCompressionCodec {
    Auto    = codec_ids::AUTO,
    Deflate = codec_ids::DEFLATE,
    Lz4     = codec_ids::LZ4,
    Zstd    = codec_ids::ZSTD,
}

/// Strategy choices for encoder metadata
#[pyclass(name = "Strategy", eq, eq_int)]
#[repr(u16)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, TryFromPrimitive)]
pub enum PyStrategy {
    Auto       = strategy_ids::AUTO,
    Sequential = strategy_ids::SEQUENTIAL,
    Parallel   = strategy_ids::PARALLEL,
}

/// Cipher suites
#[pyclass(name = "CipherSuite", eq, eq_int)]
#[repr(u16)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, TryFromPrimitive)]
pub enum PyCipherSuite {
    Aes256Gcm        = cipher_ids::AES256_GCM,
    Chacha20Poly1305 = cipher_ids::CHACHA20_POLY1305,
}

/// HKDF PRF choices
#[pyclass(name = "HkdfPrf", eq, eq_int)]
#[repr(u16)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, TryFromPrimitive)]
pub enum PyHkdfPrf {
    Sha256  = prf_ids::SHA256,
    Sha512  = prf_ids::SHA512,
    Sha3_256= prf_ids::SHA3_256,
    Sha3_512= prf_ids::SHA3_512,
    Blake3K = prf_ids::BLAKE3K,
}

/// Algorithm profile bundles cipher + PRF combinations
#[pyclass(name = "AlgProfile", eq, eq_int)]
#[repr(u16)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, TryFromPrimitive)]
pub enum PyAlgProfile {
    Aes256GcmHkdfSha256         = alg_profile_ids::AES256_GCM_HKDF_SHA256,
    Aes256GcmHkdfSha512         = alg_profile_ids::AES256_GCM_HKDF_SHA512,
    Chacha20Poly1305HkdfSha256  = alg_profile_ids::CHACHA20_POLY1305_HKDF_SHA256,
    Chacha20Poly1305HkdfSha512  = alg_profile_ids::CHACHA20_POLY1305_HKDF_SHA512,
    Chacha20Poly1305HkdfBlake3K = alg_profile_ids::CHACHA20_POLY1305_HKDF_BLAKE3K,
}

/// AAD domain identifiers
#[pyclass(name = "AadDomain", eq, eq_int)]
#[repr(u16)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, TryFromPrimitive)]
pub enum PyAadDomain {
    Generic      = aad_domain_ids::GENERIC,
    FileEnvelope = aad_domain_ids::FILE_ENVELOPE,
    PipeEnvelope = aad_domain_ids::PIPE_ENVELOPE,
}

#[pyclass(name = "HeaderV1")]
#[derive(Debug, Clone)]
pub struct PyHeaderV1 {
    #[pyo3(get, set)]
    pub alg_profile: PyAlgProfile,
    #[pyo3(get, set)]
    pub cipher: PyCipherSuite,
    #[pyo3(get, set)]
    pub hkdf_prf: PyHkdfPrf,
    #[pyo3(get, set)]
    pub compression: PyCompressionCodec,
    #[pyo3(get, set)]
    pub strategy: PyStrategy,
    #[pyo3(get, set)]
    pub aad_domain: PyAadDomain,
    // keep raw ints for flags, sizes, etc.
    #[pyo3(get, set)]
    pub magic: [u8; 4],
    #[pyo3(get, set)]
    pub version: u16,
    #[pyo3(get, set)]
    pub flags: u16,
    #[pyo3(get, set)]
    pub chunk_size: u32,
    #[pyo3(get, set)]
    pub plaintext_size: u64,
    #[pyo3(get, set)]
    pub crc32: u32,
    #[pyo3(get, set)]
    pub dict_id: u32,
    #[pyo3(get, set)]
    pub salt: [u8; 16],
    #[pyo3(get, set)]
    pub key_id: u32,
    #[pyo3(get, set)]
    pub parallel_hint: u32,
    #[pyo3(get, set)]
    pub enc_time_ns: u64,
    #[pyo3(get, set)]
    pub reserved: [u8; 8],
}

#[pymethods]
impl PyHeaderV1 {
    #[classattr]
    const LEN: usize = HeaderV1::LEN;

    #[new]
    pub fn new(
        magic          : [u8; 4],
        version        : u16,
        alg_profile    : PyAlgProfile,
        cipher         : PyCipherSuite,
        hkdf_prf       : PyHkdfPrf,
        compression    : PyCompressionCodec,
        strategy       : PyStrategy,
        aad_domain     : PyAadDomain,
        flags          : u16,
        chunk_size     : u32,
        plaintext_size : u64,
        crc32          : u32,
        dict_id        : u32,
        salt           : [u8; 16],
        key_id         : u32,
        parallel_hint  : u32,
        enc_time_ns    : u64,
        reserved       : [u8; 8],
    ) -> Self {
        Self {
            magic          : magic,
            version        : version,
            alg_profile    : alg_profile,
            cipher         : cipher,
            hkdf_prf       : hkdf_prf,
            compression    : compression,
            strategy       : strategy,
            aad_domain     : aad_domain,
            flags          : flags,
            chunk_size     : chunk_size,
            plaintext_size : plaintext_size,
            crc32          : crc32,
            dict_id        : dict_id,
            salt           : salt,
            key_id         : key_id,
            parallel_hint  : parallel_hint,
            enc_time_ns    : enc_time_ns,
            reserved       : reserved,
        }
    }
}

impl From<PyHeaderV1> for HeaderV1 {
    fn from(h: PyHeaderV1) -> Self {
        HeaderV1 {
            magic: h.magic,
            version: h.version,
            alg_profile: h.alg_profile as u16,
            cipher: h.cipher as u16,
            hkdf_prf: h.hkdf_prf as u16,
            compression: h.compression as u16,
            strategy: h.strategy as u16,
            aad_domain: h.aad_domain as u16,
            flags: h.flags,
            chunk_size: h.chunk_size,
            plaintext_size: h.plaintext_size,
            crc32: h.crc32,
            dict_id: h.dict_id,
            salt: h.salt,
            key_id: h.key_id,
            parallel_hint: h.parallel_hint,
            enc_time_ns: h.enc_time_ns,
            reserved: h.reserved,
        }
    }
}

impl From<HeaderV1> for PyHeaderV1 {
    fn from(h: HeaderV1) -> Self {
        Self {
            magic: h.magic,
            version: h.version,
            alg_profile: PyAlgProfile::try_from(h.alg_profile).unwrap(),
            cipher: PyCipherSuite::try_from(h.cipher).unwrap(),
            hkdf_prf: PyHkdfPrf::try_from(h.hkdf_prf).unwrap(),
            compression: PyCompressionCodec::try_from(h.compression).unwrap(),
            strategy: PyStrategy::try_from(h.strategy).unwrap(),
            aad_domain: PyAadDomain::try_from(h.aad_domain).unwrap(),
            flags: h.flags,
            chunk_size: h.chunk_size,
            plaintext_size: h.plaintext_size,
            crc32: h.crc32,
            dict_id: h.dict_id,
            salt: h.salt,
            key_id: h.key_id,
            parallel_hint: h.parallel_hint,
            enc_time_ns: h.enc_time_ns,
            reserved: h.reserved,
        }
    }
}

#[pymodule(name = "headers")]
pub fn register_headers(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {

    m.add_class::<PyCompressionCodec>()?;
    m.add_class::<PyStrategy>()?;
    m.add_class::<PyCipherSuite>()?;
    m.add_class::<PyHkdfPrf>()?;
    m.add_class::<PyAlgProfile>()?;
    m.add_class::<PyAadDomain>()?;
    m.add_class::<PyHeaderV1>()?;

    Ok(())
}
