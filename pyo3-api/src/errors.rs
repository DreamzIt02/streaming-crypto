// ## 📝 pyo3-api/src/errors.rs

use pyo3::prelude::*;
use pyo3::exceptions::PyException;
use core_api::types::StreamError;

#[pyclass(name = "CryptoError", extends=PyException)]
#[derive(Debug)]
pub struct PyCryptoError {
    #[pyo3(get)]
    pub code: String,
    #[pyo3(get)]
    pub message: String,
}

// ✅ Convert StreamError → PyCryptoError

impl From<StreamError> for PyCryptoError {
    fn from(err: StreamError) -> Self {
        macro_rules! simple {
            (code: $code:expr, message: $msg:expr) => {
                PyCryptoError { code: $code.into(), message: $msg }
            };
        }

        use StreamError::*;
        match err {
            // I/O and validation
            Io(msg)                        => simple!(code: "Io",             message: msg),
            IoError(kind, msg)  => simple!(code: "IoError",        message: format!("{:?}: {}", kind, msg)),
            Validation(msg)                => simple!(code: "Validation",     message: msg),
            FormatError(msg)               => simple!(code: "FormatError",    message: msg),
            PipelineError(msg)       => simple!(code: "PipelineError",  message: msg.into()),

            // Channel and threading
            ChannelSend                            => simple!(code: "ChannelSend",    message: "channel send error".into()),
            ChannelRecv                            => simple!(code: "ChannelRecv",    message: "channel receive error".into()),
            ThreadPanic                            => simple!(code: "ThreadPanic",    message: "thread panic".into()),

            // Worker errors
            Aad(e)                       => simple!(code: "Aad",            message: e.to_string()),
            Header(e)                 => simple!(code: "Header",         message: e.to_string()),
            SegmentWorker(e)   => simple!(code: "SegmentWorker",  message: e.to_string()),
            FrameWorker(e)       => simple!(code: "FrameWorker",    message: e.to_string()),
            CompressionWorker(e)=> simple!(code: "CompressionWorker", message: e.to_string()),

            // Data unit errors
            Segment(e)               => simple!(code: "Segment",        message: e.to_string()),
            Frame(e)                   => simple!(code: "Frame",          message: e.to_string()),

            // Crypto and compression
            Crypto(e)                => simple!(code: "Crypto",         message: e.to_string()),
            Compression(e)      => simple!(code: "Compression",    message: e.to_string()),
            Nonce(e)                  => simple!(code: "Nonce",          message: e.to_string()),
        }
    }
}

// ✅ Convert PyCryptoError → PyErr (Python exception)
impl From<PyCryptoError> for PyErr {
    fn from(err: PyCryptoError) -> PyErr {
        PyErr::new::<PyCryptoError, _>((err.code, err.message))
    }
}

#[pymodule(name = "errors")]
pub fn register_errors(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Register class
    m.add_class::<PyCryptoError>()?;
    Ok(())
}