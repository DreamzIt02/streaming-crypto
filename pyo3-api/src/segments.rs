use core_api::segmenting::{SegmentHeader, types::SegmentFlags};
use pyo3::prelude::*;

use core_api::types::StreamError as CoreStreamError;
use crate::PyStreamError;

#[pyclass(name = "SegmentFlags")]
#[derive(Clone, Copy, Debug)]
pub struct PySegmentFlags(pub SegmentFlags);

#[pymethods]
impl PySegmentFlags {

    #[classattr]
    const FINAL_SEGMENT: Self = Self(SegmentFlags::FINAL_SEGMENT);

    #[classattr]
    const COMPRESSED: Self    = Self(SegmentFlags::COMPRESSED);

    #[classattr]
    const RESUMED: Self       = Self(SegmentFlags::RESUMED);

    #[classattr]
    const RESERVED: Self      = Self(SegmentFlags::RESERVED);

    #[getter]
    fn value(&self) -> u16 {
        self.0.bits()
    }

    fn __int__(&self) -> u16 {
        self.0.bits()
    }

    fn __or__(&self, other: &Self) -> Self {
        Self(self.0 | other.0)
    }

    fn __and__(&self, other: &Self) -> Self {
        Self(self.0 & other.0)
    }

    fn __contains__(&self, other: &Self) -> bool {
        self.0.contains(other.0)
    }

    fn __repr__(&self) -> String {
        format!("SegmentFlags({:#06x})", self.0.bits())
    }
}
// 2️⃣ Convert between Rust flags and Python flags

impl From<SegmentFlags> for PySegmentFlags {
    fn from(v: SegmentFlags) -> Self {
        Self(v)
    }
}

impl From<PySegmentFlags> for SegmentFlags {
    fn from(v: PySegmentFlags) -> Self {
        v.0
    }
}

#[pyclass(name="SegmentHeader")]
pub struct PySegmentHeader {
    #[pyo3(get)]
    pub segment_index: u32,

    #[pyo3(get)]
    pub bytes_len: u32,

    #[pyo3(get)]
    pub wire_len: u32,

    #[pyo3(get)]
    pub wire_crc32: u32,

    #[pyo3(get)]
    pub frame_count: u32,

    #[pyo3(get)]
    pub digest_alg: u16,

    #[pyo3(get)]
    pub flags: PySegmentFlags,

    #[pyo3(get)]
    pub header_crc32: u32,
}

// # 1️⃣ `From<SegmentHeader> for PySegmentHeader`

impl From<SegmentHeader> for PySegmentHeader {
    fn from(h: SegmentHeader) -> Self {
        Self {
            segment_index: h.segment_index(),
            bytes_len: h.bytes_len(),
            wire_len: h.wire_len(),
            wire_crc32: h.wire_crc32(),
            frame_count: h.frame_count(),
            digest_alg: h.digest_alg(),
            flags: PySegmentFlags(h.flags()),
            header_crc32: h.header_crc32(),
        }
    }
}

#[pymethods]
impl PySegmentHeader {
    #[classattr]
    const LEN: usize = SegmentHeader::LEN;

    #[staticmethod]
    pub fn from_bytes(data: &[u8]) -> PyResult<Self> {
        let h = SegmentHeader::from_bytes(data)
            .map_err(|e| PyStreamError::from(CoreStreamError::Segment(e)))?;

        Ok(Self {
            segment_index: h.segment_index(),
            bytes_len: h.bytes_len(),
            wire_len: h.wire_len(),
            wire_crc32: h.wire_crc32(),
            frame_count: h.frame_count(),
            digest_alg: h.digest_alg(),
            flags: PySegmentFlags(h.flags()),
            header_crc32: h.header_crc32(),
        })
    }

}

#[pymodule(name = "segments")]
pub fn register_segments(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PySegmentFlags>()?;
    m.add_class::<PySegmentHeader>()?;
    Ok(())
}

// # 5️⃣ Python usage (beautiful API)

// ```python
// from streaming_crypto import SegmentHeader, SegmentFlags

// header = SegmentHeader.from_bytes(data)

// if header.flags & SegmentFlags.COMPRESSED:
//     print("compressed")

// if header.flags & SegmentFlags.FINAL_SEGMENT:
//     print("last segment")
// ```
