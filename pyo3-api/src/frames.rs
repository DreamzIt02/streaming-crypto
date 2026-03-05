use core_api::framing::{FrameHeader, FrameType};
use pyo3::prelude::*;
use num_enum::TryFromPrimitive;

#[pyclass(name = "FrameType", eq, eq_int)]
#[repr(u16)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, TryFromPrimitive)]
pub enum PyFrameType {
    Data       = FrameType::Data as u16,
    Terminator = FrameType::Terminator as u16,
    Digest     = FrameType::Digest as u16,
}

// # 2️⃣ Conversion between Rust and Python enums

// ### Core → Python
impl From<FrameType> for PyFrameType {
    fn from(v: FrameType) -> Self {
        match v {
            FrameType::Data         => PyFrameType::Data,
            FrameType::Terminator   => PyFrameType::Terminator,
            FrameType::Digest       => PyFrameType::Digest,
        }
    }
}

// ### Python → Core
impl From<PyFrameType> for FrameType {
    fn from(v: PyFrameType) -> Self {
        match v {
            PyFrameType::Data       => FrameType::Data,
            PyFrameType::Terminator => FrameType::Terminator,
            PyFrameType::Digest     => FrameType::Digest,
        }
    }
}

// # 3️⃣ Optional: expose numeric value
#[pymethods]
impl PyFrameType {

    #[getter]
    fn value(&self) -> u16 {
        *self as u16
    }

    fn __int__(&self) -> u16 {
        *self as u16
    }

    fn __repr__(&self) -> String {
        format!("FrameType::{:?}", self)
    }
}

#[pyclass(name = "FrameHeader")]
#[derive(Debug, Clone)]
pub struct PyFrameHeader {
    #[pyo3(get)]
    pub segment_index: u32,

    #[pyo3(get)]
    pub frame_index: u32,

    #[pyo3(get)]
    pub frame_type: PyFrameType,

    #[pyo3(get)]
    pub plaintext_len: u32,

    #[pyo3(get)]
    pub ciphertext_len: u32,
}

#[pymethods]
impl PyFrameHeader {
    #[classattr]
    const LEN: usize = FrameHeader::LEN;

    fn __repr__(&self) -> String {
        format!(
            "FrameHeader(segment_index={}, frame_index={}, frame_type={:?}, plaintext_len={}, ciphertext_len={})",
            self.segment_index,
            self.frame_index,
            self.frame_type,
            self.plaintext_len,
            self.ciphertext_len
        )
    }
}

// # 3️⃣ Conversion: Core → Python
impl From<FrameHeader> for PyFrameHeader {
    fn from(h: FrameHeader) -> Self {
        Self {
            segment_index: h.segment_index(),
            frame_index: h.frame_index(),
            frame_type: h.frame_type().into(),
            plaintext_len: h.plaintext_len(),
            ciphertext_len: h.ciphertext_len(),
        }
    }
}

#[pymodule(name = "frames")]
pub fn register_frames(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyFrameType>()?;
    m.add_class::<PyFrameHeader>()?;
    Ok(())
}