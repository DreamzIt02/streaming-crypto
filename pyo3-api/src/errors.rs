// ## 📝 pyo3-api/src/errors.rs

use pyo3::prelude::*;
use pyo3::{create_exception, exceptions::PyException};
use core_api::types::StreamError as CoreStreamError;

// Define a proper Python exception type (different name!)
create_exception!(errors, StreamError, PyException);

// Structured mapping stays
#[derive(Debug)]
pub struct PyStreamError {
    pub code: String,
    pub message: String,
}

// ✅ Convert CoreStreamError → PyStreamError
impl From<CoreStreamError> for PyStreamError {
    fn from(err: CoreStreamError) -> Self {
        macro_rules! simple {
            (code: $code:expr, message: $msg:expr) => {
                PyStreamError { code: $code.into(), message: $msg }
            };
        }

        use CoreStreamError::*;
        match err {
            Io(msg)                         => simple!(code: "Io",             message: msg),
            IoError(kind, msg)   => simple!(code: "IoError",        message: format!("{:?}: {}", kind, msg)),
            Validation(msg)                 => simple!(code: "Validation",     message: msg),
            FormatError(msg)                => simple!(code: "FormatError",    message: msg),
            PipelineError(msg)        => simple!(code: "PipelineError",  message: msg.into()),
            ChannelSend                             => simple!(code: "ChannelSend",    message: "channel send error".into()),
            ChannelRecv                             => simple!(code: "ChannelRecv",    message: "channel receive error".into()),
            ThreadPanic                             => simple!(code: "ThreadPanic",    message: "thread panic".into()),

            Aad(e)                        => simple!(code: "Aad",            message: e.to_string()),
            Header(e)                  => simple!(code: "Header",         message: e.to_string()),
            SegmentWorker(e)    => simple!(code: "SegmentWorker",  message: e.to_string()),
            FrameWorker(e)        => simple!(code: "FrameWorker",    message: e.to_string()),
            
            CompressionWorker(e) => simple!(code: "CompressionWorker", message: e.to_string()),

            Segment(e)                => simple!(code: "Segment",        message: e.to_string()),
            Frame(e)                    => simple!(code: "Frame",          message: e.to_string()),
            Crypto(e)                  => simple!(code: "Crypto",         message: e.to_string()),
            Compression(e)        => simple!(code: "Compression",    message: e.to_string()),
            Nonce(e)                    => simple!(code: "Nonce",          message: e.to_string()),
        }
    }
}

// ✅ Raise StreamError in Python using the mapped fields
// ✅ Convert PyStreamError → PyErr (Python exception) (Version B from errors.md)
impl From<PyStreamError> for PyErr {
    fn from(err: PyStreamError) -> PyErr {
        Python::with_gil(|py| {
            let exc = StreamError::new_err((err.code.clone(), err.message.clone()));

            // Get the underlying Python exception object
            let obj = exc.clone_ref(py).into_value(py);

            // Dynamically attach code and message attributes
            obj.setattr::<&str, String>(py, "code", err.code).ok();
            obj.setattr::<&str, String>(py, "message", err.message).ok();

            exc
        })
    }
}

#[pymodule(name = "errors")]
pub fn register_errors(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Register the exception type
    m.add("StreamError", py.get_type_bound::<StreamError>())?;
    Ok(())
}
