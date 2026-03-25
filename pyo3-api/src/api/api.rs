
// ## 📝 pyo3-api/src/api.rs

use core_api::InputSource;
use core_api::crypto::DigestAlg;
use core_api::parallelism::ParallelismConfig;
use pyo3::prelude::*;
use pyo3::types::{PyByteArray, PyBytes};
use pyo3::{PyObject, PyResult, Python, pyfunction};

use core_api::stream_v2::{ApiConfig, DecryptParams, EncryptParams, MasterKey, decrypt_stream_v2, encrypt_stream_v2};
use crate::api::api_io::{classify_py_io, to_core_input, to_core_output};
use crate::{PyDigestAlg, PyInputSource, PyParallelismConfig, PyStreamError};
use crate::{headers::PyHeaderV1, telemetry::PyTelemetrySnapshot};

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
            header: c.header.clone().into(),
            dict: c.dict.as_deref(),
        }
    }
}

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
    fn from(_c: PyDecryptParams) -> Self {
        DecryptParams {
        }
    }
}

#[pyclass(name = "ApiConfig")]
#[derive(Debug, Clone)]
pub struct PyApiConfig {
    #[pyo3(get, set)]
    pub with_buf: Option<bool>,
    #[pyo3(get, set)]
    pub collect_metrics: Option<bool>,

    /// Supported digest algorithms (extensible).
    #[pyo3(get, set)]
    pub alg: Option<PyDigestAlg>,

    /// Parallelism configuration.
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
        Self {
            with_buf,
            collect_metrics,
            alg,
            parallelism
        }
    }
}

impl From<PyApiConfig> for ApiConfig {
    fn from(c: PyApiConfig) -> Self {
        
        ApiConfig {
            with_buf: c.with_buf,
            collect_metrics: c.collect_metrics,
            alg: c.alg.map(DigestAlg::from),          // ✅ converts Option<PyDigestAlg> → Option<DigestAlg>
            parallelism: c.parallelism.map(ParallelismConfig::from),
        }

    }
}

// # 4️⃣ Entry point (final pipeline)

// ```
// Python objects
//      ↓
// PyInputSource / PyOutputSink   (PyO3 layer)
//      ↓
// InputSource / OutputSink       (core-api)
//      ↓
// encrypt_stream_v2()            (Rust pipeline)
// ```

#[pyfunction(name = "encrypt_stream_v2")]
pub fn py_encrypt_stream_v2(
    py: Python,
    input: PyObject,
    output: PyObject,
    params: PyEncryptParams,
    config: PyApiConfig,
) -> PyResult<PyTelemetrySnapshot> {

    let master_key = MasterKey::new(params.clone().master_key);
    let params = EncryptParams::from(&params);
    let config: ApiConfig = ApiConfig::from(config);

    let (py_input, py_output) = classify_py_io(py, input, output, Some(true))?;
    let output_sink = to_core_output(py, py_output, Some(true))?;

    // Bind the lifetime of the slice to THIS scope (GIL is held here)
    let result = match py_input {

        // ✅ Zero-copy: slice lives for this scope, GIL is still held,
        // but we process entirely on the Rust side without touching Python
        PyInputSource::Bytes(ref obj) => {
            let slice: &[u8] = obj.bind(py).downcast::<PyBytes>()?.as_bytes();
            // 🚀 GIL released — slice is valid because Python object is
            // kept alive by `obj` which is still in scope above
            py.allow_threads(|| {
                encrypt_stream_v2(
                    InputSource::Memory(slice),
                    output_sink,
                    &master_key,
                    params,
                    config,
                )
            })
        }

        PyInputSource::ByteArray(ref obj) => {
            // ⚠️ ByteArray is mutable — Python could resize it while
            // Rust reads it. Either refuse ByteArray for Memory path,
            // or copy here only (small, justified tradeoff)
            let bytes: Vec<u8> = unsafe {
                obj.bind(py).downcast::<PyByteArray>()?.as_bytes().to_vec()
            };
            // In api.rs — ByteArray copy path
            crate::increment_input_copies();
            py.allow_threads(|| {
                encrypt_stream_v2(
                    InputSource::Memory(&bytes),
                    output_sink,
                    &master_key,
                    params,
                    config,
                )
            })
        }

        _ => {
            // File / Reader path: normal flow
            let input_src = to_core_input(py, py_input, Some(params.header.chunk_size as usize), Some(true))?;
            py.allow_threads(|| {
                encrypt_stream_v2(input_src, output_sink, &master_key, params, config)
            })
        }
    };

    match result {
        Ok(snapshot) => Ok(snapshot.into()),
        Err(e) => Err(PyErr::from(PyStreamError::from(e))),
    }
}

#[pyfunction(name = "decrypt_stream_v2")]
pub fn py_decrypt_stream_v2(
    py: Python,
    input: PyObject,
    output: PyObject,
    params: PyDecryptParams,
    config: PyApiConfig,
) -> PyResult<PyTelemetrySnapshot> {

    let master_key = MasterKey::new(params.clone().master_key);
    let params = DecryptParams::from(params);
    let config: ApiConfig = ApiConfig::from(config);

    let (py_input, py_output) = classify_py_io(py, input, output, Some(true))?;
    let output_sink = to_core_output(py, py_output, Some(true))?;

    let result = match py_input {

        // ✅ Zero-copy: PyBytes is immutable, safe to read without GIL
        PyInputSource::Bytes(ref obj) => {
            let slice: &[u8] = obj.bind(py).downcast::<PyBytes>()?.as_bytes();
            py.allow_threads(|| {
                decrypt_stream_v2(
                    InputSource::Memory(slice),
                    output_sink,
                    &master_key,
                    params,
                    config,
                )
            })
        }

        // ⚠️ PyByteArray is mutable — copy is justified to avoid races
        PyInputSource::ByteArray(ref obj) => {
            let bytes: Vec<u8> = unsafe {
                obj.bind(py).downcast::<PyByteArray>()?.as_bytes().to_vec()
            };
            py.allow_threads(|| {
                decrypt_stream_v2(
                    InputSource::Memory(&bytes),
                    output_sink,
                    &master_key,
                    params,
                    config,
                )
            })
        }

        // File / Reader: normal flow, no chunk_size needed for decrypt
        _ => {
            let input_src = to_core_input(py, py_input, None, Some(true))?;
            py.allow_threads(|| {
                decrypt_stream_v2(
                    input_src,
                    output_sink,
                    &master_key,
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

// # 5️⃣ Why this architecture is excellent

//     ✔ `core-api` remains **pure Rust**
//     ✔ `pyo3-api` isolates Python handling
//     ✔ **no lifetime headaches**
//     ✔ **zero-copy for Python bytes**
//     ✔ easy to extend for `numpy`, `memoryview`, `mmap`
//     ✔ pipeline runs **GIL-free**

//     Flow:

//     ```
//     Python objects
//         ↓
//     PyInputSource / PyOutputSink
//         ↓
//     InputSource / OutputSink
//         ↓
//     allow_threads()
//         ↓
//     Rust streaming pipeline
//     ```

// ## ✅ Python Usage Examples

// ### Memory → Memory
// ```python
// snap = scp.py_encrypt_stream_v2(b"hello", None, b"key", header, config)
// print("Ciphertext:", snap.output)

// snap = scp.py_decrypt_stream_v2(snap.output, None, b"key", config)
// print("Plaintext:", snap.output)
// ```

// ### File → File
// ```python
// snap = scp.py_encrypt_stream_v2("input.txt", "output.enc", b"key", header, config)
// snap = scp.py_decrypt_stream_v2("output.enc", "decrypted.txt", b"key", config)
// ```

// ### File‑like Objects with Zero‑Copy
// ```python
// import io

// class MyReader(io.BytesIO):
//     def readinto(self, b: bytearray) -> int:
//         return super().readinto(b)

// class MyWriter(io.BytesIO):
//     def writeinto(self, b: bytearray) -> int:
//         self.write(b)
//         return len(b)

// inp = MyReader(b"hello world")
// out = MyWriter()

// snap = scp.py_encrypt_stream_v2(inp, out, b"key", header, config)
// print("Ciphertext:", out.getvalue())

// out2 = MyWriter()
// snap = scp.py_decrypt_stream_v2(io.BytesIO(out.getvalue()), out2, b"key", config)
// print("Decrypted:", out2.getvalue())
// ```

// ## 🚀 Production‑Ready Symmetry
// - Both encrypt and decrypt now auto‑detect:
//   - `bytes` → memory
//   - `str` → file path
//   - `.readinto` / `.writeinto` → zero‑copy adapters
//   - `.read` / `.write` → fallback adapters
// - This ensures maximum performance and natural Python ergonomics.
