
use std::io::Cursor;
use pyo3::{prelude::*, types::PyTuple};

use core_api::io::PayloadReader;

pub mod io;
pub mod io_ht;
pub mod io_readinto;
pub mod io_writeinto;

pub use io::*;
pub use io_ht::*;
pub use io_readinto::*;
pub use io_writeinto::*;

use crate::{PyHeaderV1, PyStreamError};

#[pyclass(name="PayloadReader")]
pub struct PyPayloadReader {
    _inner: PayloadReader<Cursor<Vec<u8>>>,
}

#[pymethods]
impl PyPayloadReader {
    #[staticmethod]
    pub fn with_header(py: Python<'_>, data: Vec<u8>) -> PyResult<PyObject> {
        let cursor = Cursor::new(data);

        match PayloadReader::with_header(cursor) {
            Ok((header, reader)) => {
                let py_header = PyHeaderV1::from(header).into_py(py);
                let py_reader = PyPayloadReader { _inner: reader }.into_py(py);

                Ok(PyTuple::new_bound(py, &[py_header, py_reader]).into())
            }
            Err(e) => Err(PyErr::from(PyStreamError::from(e))),
        }
    }
}

#[pymodule(name = "io")]
pub fn register_io(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {

    // Register class
    m.add_class::<PyPayloadReader>()?;

    Ok(())
}
