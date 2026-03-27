
// ## 📝 pyo3-api/src/api.rs

use core_api::InputSource;
use core_api::crypto::DigestAlg;
use core_api::parallelism::ParallelismConfig;
use pyo3::prelude::*;
use pyo3::types::{PyByteArray, PyBytes};
use pyo3::{PyObject, PyResult, Python, pyfunction};

use core_api::stream::{MasterKey};
use core_api::v3::stream::{core::{ApiConfig, EncryptParams, DecryptParams}, encrypt_stream_v3, decrypt_stream_v3};
use crate::{
    PyDigestAlg, PyParallelismConfig, PyStreamError, headers::PyHeaderV1, telemetry::PyTelemetrySnapshot, 
    increment_input_copies,
    v3::{api::{PyInputSource, classify_py_io, to_core_input, to_core_output}}
};

// ============================================================
// PyO3 bindings — API v3
//
// Architecture:
//   Python objects
//        ↓
//   PyInputSource  (classify_py_io — same as v2)
//        ↓
//   InputSource    (constructed directly here — NOT via to_core_input)
//        ↓
//   encrypt_stream_v3 / decrypt_stream_v3  (Rust pipeline)
//
// Key difference from v2:
//   v2 called to_core_input() which opened/boxed the input before passing
//   to the pipeline. v3 constructs InputSource directly so the pipeline's
//   parallel reader workers (spawn_encrypt_readers_scoped /
//   spawn_decrypt_readers_scoped) can open and dispatch it themselves —
//   enabling parallel pread for File and zero-copy slice for Memory.
// ============================================================

// ============================================================
// EncryptParams
// ============================================================

#[pyclass(name = "EncryptParams")]
#[derive(Clone, Debug)]
pub struct PyEncryptParams {
    #[pyo3(get, set)]
    pub header: PyHeaderV1,
    #[pyo3(get, set)]
    pub dict: Option<Vec<u8>>,
    #[pyo3(get, set)]
    pub master_key: Vec<u8>,
}

#[pymethods]
impl PyEncryptParams {
    #[new]
    #[pyo3(signature = (master_key, header, dict=None))]
    fn new(master_key: Vec<u8>, header: PyHeaderV1, dict: Option<Vec<u8>>) -> Self {
        Self { header, dict, master_key }
    }
}

impl<'a> From<&'a PyEncryptParams> for EncryptParams<'a> {
    fn from(c: &'a PyEncryptParams) -> Self {
        EncryptParams {
            header      : c.header.clone().into(),
            dict        : c.dict.as_deref(),
            master_key  : MasterKey::new(c.master_key.clone()),
        }
    }
}

// ============================================================
// DecryptParams
// ============================================================

#[pyclass(name = "DecryptParams")]
#[derive(Clone, Debug)]
pub struct PyDecryptParams {
    #[pyo3(get, set)]
    pub master_key: Vec<u8>,
}

#[pymethods]
impl PyDecryptParams {
    #[new]
    fn new(master_key: Vec<u8>) -> Self {
        Self { master_key }
    }
}

impl From<PyDecryptParams> for DecryptParams {
    fn from(c: PyDecryptParams) -> Self {
        DecryptParams {
            master_key  : MasterKey::new(c.master_key.clone()),
        }
    }
}

// ============================================================
// ApiConfig
// ============================================================

#[pyclass(name = "ApiConfig")]
#[derive(Debug, Clone)]
pub struct PyApiConfig {
    #[pyo3(get, set)]
    pub with_buf: Option<bool>,
    #[pyo3(get, set)]
    pub collect_metrics: Option<bool>,
    #[pyo3(get, set)]
    pub alg: Option<PyDigestAlg>,
    #[pyo3(get, set)]
    pub parallelism: Option<PyParallelismConfig>,
}

#[pymethods]
impl PyApiConfig {
    #[new]
    #[pyo3(signature = (with_buf=None, collect_metrics=None, alg=None, parallelism=None))]
    pub fn new(
        with_buf: Option<bool>,
        collect_metrics: Option<bool>,
        alg: Option<PyDigestAlg>,
        parallelism: Option<PyParallelismConfig>,
    ) -> Self {
        Self { with_buf, collect_metrics, alg, parallelism }
    }
}

impl From<PyApiConfig> for ApiConfig {
    fn from(c: PyApiConfig) -> Self {
        ApiConfig {
            with_buf: c.with_buf,
            collect_metrics: c.collect_metrics,
            alg: c.alg.map(DigestAlg::from),
            parallelism: c.parallelism.map(ParallelismConfig::from),
        }
    }
}

// ============================================================
// encrypt_stream_v3
//
// Input handling:
//
//   PyBytes     → InputSource::Memory(&[u8])
//                 Zero-copy. PyBytes is immutable + ref-counted.
//                 The Py<PyBytes> object stays alive in this scope
//                 while allow_threads() runs, keeping the slice valid.
//
//   PyByteArray → Vec<u8> copy → InputSource::Memory(&bytes)
//                 PyByteArray is mutable — Python could resize it while
//                 Rust parallel workers are reading. Copy is small and
//                 justified to avoid the data race.
//
//   File/Path   → InputSource::File(PathBuf)
//                 Forwarded raw. The pipeline opens it with Arc<File>
//                 and dispatches parallel pread workers itself.
//
//   Reader      → InputSource::Reader(Box<dyn Read + Send>)
//                 Forwarded raw. Single stream reader worker in pipeline.
// ============================================================

#[pyfunction(name = "encrypt_stream_v3")]
pub fn py_encrypt_stream_v3(
    py: Python,
    input: PyObject,
    output: PyObject,
    params: PyEncryptParams,
    config: PyApiConfig,
) -> PyResult<PyTelemetrySnapshot> {

    let params = EncryptParams::from(&params);
    let config: ApiConfig = ApiConfig::from(config);

    let (py_input, py_output) = classify_py_io(py, input, output, Some(true))?;
    let output_sink = to_core_output(py, py_output, Some(true))?;

    let result = match py_input {

        // ✅ Zero-copy: PyBytes is immutable + ref-counted.
        // `obj` stays alive in this scope → slice is valid for the duration
        // of allow_threads(). The pipeline's memory reader workers borrow
        // it as &'scope [u8] inside the crossbeam scope.
        PyInputSource::Bytes(ref obj) => {
            let slice: &[u8] = obj.bind(py).downcast::<PyBytes>()?.as_bytes();
            py.allow_threads(|| {
                encrypt_stream_v3(
                    InputSource::Memory(slice),
                    output_sink,
                    params,
                    config,
                )
            })
        }

        // ⚠️ PyByteArray is mutable — Python could resize it while parallel
        // workers read. Copy to Vec<u8> here; the pipeline then gets a
        // stable &[u8] with no race risk.
        PyInputSource::ByteArray(ref obj) => {
            let bytes: Vec<u8> = unsafe {
                obj.bind(py).downcast::<PyByteArray>()?.as_bytes().to_vec()
            };
            increment_input_copies();
            py.allow_threads(|| {
                encrypt_stream_v3(
                    InputSource::Memory(&bytes),
                    output_sink,
                    params,
                    config,
                )
            })
        }

        // ✅ File path: forward as InputSource::File(PathBuf).
        // The pipeline opens it with Arc<File> and spawns parallel
        // pread workers (spawn_pread_reader_enc). No boxing here.
        PyInputSource::File(path) => {
            py.allow_threads(|| {
                encrypt_stream_v3(
                    InputSource::File(path),
                    output_sink,
                    params,
                    config,
                )
            })
        }

        // ✅ Reader fallback: box the Python reader as Box<dyn Read + Send>
        // and forward as InputSource::Reader. The pipeline's stream reader
        // worker serialises reads through an Arc<Mutex<_>>.
        // chunk_size is NOT passed here — the pipeline reads it from
        // crypto.base.segment_size, same as File path.
        _ => {
            let input_src = to_core_input(py, py_input, None, Some(true))?;
            py.allow_threads(|| {
                encrypt_stream_v3(
                    input_src,
                    output_sink,
                    params,
                    config,
                )
            })
        }
    };

    match result {
        Ok(snapshot) => Ok(snapshot.into()),
        Err(e) => Err(PyErr::from(PyStreamError::from(e))),
    }
}

// ============================================================
// decrypt_stream_v3
//
// Header stripping — no longer a PyO3 concern:
//   v2 called PayloadReader::with_header() here to advance the
//   reader past the stream header before passing it down.
//   v3 does not. decrypt_stream_v3() calls decrypt_read_header()
//   internally on the raw InputSource:
//     Memory → Cursor slices past header, returns &data[pos..]
//     File   → opens file, reads header, returns same PathBuf
//               (spawn_pread_reader_dec uses HeaderV1::LEN as
//               the pread start_offset, skipping the header)
//     Reader → advances reader, returns same Box<dyn Read+Send>
//
//   All three variants handled cleanly inside Rust. PyO3 forwards
//   the raw InputSource and never touches the header.
// ============================================================

#[pyfunction(name = "decrypt_stream_v3")]
pub fn py_decrypt_stream_v3(
    py: Python,
    input: PyObject,
    output: PyObject,
    params: PyDecryptParams,
    config: PyApiConfig,
) -> PyResult<PyTelemetrySnapshot> {

    let params = DecryptParams::from(params);
    let config: ApiConfig = ApiConfig::from(config);

    let (py_input, py_output) = classify_py_io(py, input, output, Some(true))?;
    let output_sink = to_core_output(py, py_output, Some(true))?;

    let result = match py_input {

        // ✅ Zero-copy: PyBytes is immutable.
        // decrypt_read_header() slices past the header internally:
        //   InputSource::Memory(data) → cursor.position() → &data[pos..]
        // No PyO3 involvement needed.
        PyInputSource::Bytes(ref obj) => {
            let slice: &[u8] = obj.bind(py).downcast::<PyBytes>()?.as_bytes();
            py.allow_threads(|| {
                decrypt_stream_v3(
                    InputSource::Memory(slice),
                    output_sink,
                    params,
                    config,
                )
            })
        }

        // ⚠️ PyByteArray mutable — copy justified, same reasoning as encrypt.
        // decrypt_read_header() will slice past the header from the copy.
        PyInputSource::ByteArray(ref obj) => {
            let bytes: Vec<u8> = unsafe {
                obj.bind(py).downcast::<PyByteArray>()?.as_bytes().to_vec()
            };
            increment_input_copies();
            py.allow_threads(|| {
                decrypt_stream_v3(
                    InputSource::Memory(&bytes),
                    output_sink,
                    params,
                    config,
                )
            })
        }

        // ✅ File path: forwarded raw.
        // decrypt_read_header() opens the file, reads the header, then
        // returns InputSource::File(path) unchanged — the decrypt pipeline's
        // pread reader skips past the header via the start_offset=HeaderV1::LEN
        // passed to spawn_pread_reader_dec.
        PyInputSource::File(path) => {
            py.allow_threads(|| {
                decrypt_stream_v3(
                    InputSource::File(path),
                    output_sink,
                    params,
                    config,
                )
            })
        }

        // ✅ Reader fallback: box and forward.
        // decrypt_read_header() advances the reader past the header bytes,
        // then returns InputSource::Reader(reader) for the pipeline.
        _ => {
            let input_src = to_core_input(py, py_input, None, Some(true))?;
            py.allow_threads(|| {
                decrypt_stream_v3(
                    input_src,
                    output_sink,
                    params,
                    config,
                )
            })
        }
    };

    match result {
        Ok(snapshot) => Ok(snapshot.into()),
        Err(e) => Err(PyErr::from(PyStreamError::from(e))),
    }
}

// ============================================================
// Notes on zero-copy expansion (memoryview / mmap)
//
// If we add PyInputSource::MemoryView or PyInputSource::Mmap
// variants in classify_py_io(), handle them like PyBytes:
//
//   PyInputSource::MemoryView(ref obj) => {
//       // PyMemoryView exposes the buffer protocol — as_bytes()
//       // is zero-copy as long as the view is contiguous and the
//       // backing object is kept alive here.
//       // If buffer.readonly == false, fall back to copy (same as
//       // ByteArray) to avoid races with mutable backing objects.
//       let buf = obj.bind(py).downcast::<PyMemoryView>()?;
//       let slice: &[u8] = buf.as_bytes()?;
//       py.allow_threads(|| encrypt_stream_v3(InputSource::Memory(slice), ...))
//   }
//
//   PyInputSource::Mmap(ref obj) => {
//       // mmap implements the buffer protocol. For both encrypt and
//       // decrypt, InputSource::Memory is strictly better than File
//       // here — the kernel mapping is already open, so page faults
//       // serve the data directly with no pread syscalls.
//       let slice: &[u8] = obj.bind(py).downcast::<PyMemoryView>()?.as_bytes()?;
//       py.allow_threads(|| encrypt_stream_v3(InputSource::Memory(slice), ...))
//   }
//
// PyByteArray zero-copy is possible only if we verify
// readonly == true via PyBuffer<u8>::get(). In practice a raw
// bytearray passed directly is always writable — copy is the
// correct default unless we add an explicit readonly check.
// ============================================================