// ## 📝 pyo3-api/src/crypto.rs

use core_api::{constants::digest_ids, crypto::DigestAlg};
use pyo3::prelude::*;
use num_enum::TryFromPrimitive;

/// Digest algorithms
#[pyclass(name = "DigestAlg", eq, eq_int)]
#[repr(u16)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, TryFromPrimitive)]
pub enum PyDigestAlg {
    Sha256   = digest_ids::SHA256,
    Sha512   = digest_ids::SHA512,
    Sha3_256 = digest_ids::SHA3_256,
    Sha3_512 = digest_ids::SHA3_512,
    Blake3   = digest_ids::BLAKE3K,// UN-KEYED Blake3
}

// Conversion from core Rust enum to PyO3 wrapper
impl From<DigestAlg> for PyDigestAlg {
    fn from(d: DigestAlg) -> Self {
        match d {
            DigestAlg::Sha256   => PyDigestAlg::Sha256,
            DigestAlg::Sha512   => PyDigestAlg::Sha512,
            DigestAlg::Sha3_256 => PyDigestAlg::Sha3_256,
            DigestAlg::Sha3_512 => PyDigestAlg::Sha3_512,
            DigestAlg::Blake3   => PyDigestAlg::Blake3,
        }
    }
}

// And the reverse, so we can go back when needed:
impl From<PyDigestAlg> for DigestAlg {
    fn from(d: PyDigestAlg) -> Self {
        match d {
            PyDigestAlg::Sha256   => DigestAlg::Sha256,
            PyDigestAlg::Sha512   => DigestAlg::Sha512,
            PyDigestAlg::Sha3_256 => DigestAlg::Sha3_256,
            PyDigestAlg::Sha3_512 => DigestAlg::Sha3_512,
            PyDigestAlg::Blake3   => DigestAlg::Blake3,
        }
    }
}

#[pymodule(name = "crypto")]
pub fn register_crypto(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Register class
    m.add_class::<PyDigestAlg>()?;
    Ok(())
}