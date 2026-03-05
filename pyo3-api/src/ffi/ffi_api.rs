
// ## 📝 pyo3-api/src/ffi_api.rs

use core_api::crypto::DigestAlg;
use core_api::parallelism::ParallelismConfig;
use pyo3::prelude::*;
use pyo3::{Bound, PyObject, PyResult, Python, exceptions::PyRuntimeError, pyfunction, pymodule, 
    types::{PyModule, PyModuleMethods}, wrap_pyfunction};

use core_api::stream_v2::{ApiConfig, DecryptParams, EncryptParams, MasterKey, decrypt_stream_v2, encrypt_stream_v2};
use crate::{PyDigestAlg, PyParallelismConfig, PyStreamError};
use crate::{headers::PyHeaderV1, telemetry::PyTelemetrySnapshot, ffi::ffi_io::py_extract_io};

#[pymodule(name = "ffi_api")]
pub fn register_api(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Register class
    m.add_class::<PyEncryptParams>()?;
    m.add_class::<PyDecryptParams>()?;
    m.add_class::<PyApiConfig>()?;

    // Register func
    m.add_function(wrap_pyfunction!(py_encrypt_stream_v2, m)?)?;
    m.add_function(wrap_pyfunction!(py_decrypt_stream_v2, m)?)?;

    Ok(())
}

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

#[pyfunction(name = "encrypt_stream_v2")]
fn py_encrypt_stream_v2(
    py: Python,
    input: PyObject,   // bytes, str path, or file-like
    output: PyObject,
    params: PyEncryptParams,
    config: PyApiConfig,
) -> PyResult<PyTelemetrySnapshot> {
    let master_key = MasterKey::new(params.clone().master_key);
    let params = EncryptParams::from(&params);
    let config: ApiConfig = ApiConfig::from(config);

    // Extract Python-side objects while GIL is held
    let (input_src, output_sink) = py_extract_io(py, input, output, Some(params.header.chunk_size as usize))?;

    // 🚀 CRITICAL: Release GIL for entire pipeline
    let result = py.allow_threads(|| {
        encrypt_stream_v2(
            input_src,
            output_sink,
            &master_key,
            params,
            config,
        )
    });

    match result {
        Ok(snapshot) => Ok(snapshot.into()),
        Err(e) => Err(PyErr::from(PyStreamError::from(e))),
    }
    
}

#[pyfunction(name = "decrypt_stream_v2")]
fn py_decrypt_stream_v2(
    py: Python,
    input: PyObject,        // bytes, str path, or file-like
    output: PyObject,
    params: PyDecryptParams,
    config: PyApiConfig,
) -> PyResult<PyTelemetrySnapshot> {
    let master_key = MasterKey::new(params.clone().master_key);
    let params = DecryptParams::from(params);
    let config: ApiConfig = ApiConfig::from(config);

    // Extract Python-side objects while GIL is held
    let (input_src, output_sink) = py_extract_io(py, input, output, None)?;

    // 🚀 CRITICAL: Release GIL for entire pipeline
    let result = py.allow_threads(|| {
        decrypt_stream_v2(
            input_src,
            output_sink,
            &master_key,
            params,
            config,
        )
    });

    match result {
        Ok(snapshot) => Ok(snapshot.into()),
        Err(e) => Err(PyErr::from(PyStreamError::from(e))),
    }
}

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
