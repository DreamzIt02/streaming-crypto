
// The fix is to make our classifiers **strict and symmetric**:  
// - `bytes`/`bytearray`/`memoryview` → always `Memory`  
// - `str` → always `File` (never mis‑classified as raw bytes)  
// - everything else → treated as file‑like `Reader`/`Writer`  

use std::{io::{Read, Write}, path::PathBuf};
use pyo3::{PyObject, PyResult, Python, types::{PyByteArray, PyByteArrayMethods, PyBytes, PyBytesMethods, PyMemoryView}};
use pyo3::types::PyAnyMethods; // ✅ bring both traits into scope

use core_api::{stream::{InputSource, OutputSink}};
use crate::{PyInputSource, PyOutputSink, io::{PyReader, PyReaderHT, PyReaderReadInto, PyWriter, PyWriterHT, PyWriterWriteInto}};

// ```rust
// dbg_detect!(debug, "Input source detected: memory (bytes)");
// ```
macro_rules! dbg_detect {
    ($debug:expr, $msg:expr) => {
        if $debug == Some(true) {
            println!("{}", $msg);
        }
    };
}

// # 2️⃣ IO classifier

pub fn classify_py_io(
    py: Python,
    input: PyObject,
    output: PyObject,
    debug: Option<bool>,
) -> PyResult<(PyInputSource, PyOutputSink)> {
    let input_obj = input.bind(py);
    let output_obj = output.bind(py);

    // ---------- INPUT ----------

    let py_input = if input_obj.is_instance_of::<PyBytes>() {
        dbg_detect!(debug, "Input source detected: memory (bytes)");
        PyInputSource::Bytes(input)
    }
    else if input_obj.is_instance_of::<PyByteArray>() {
        dbg_detect!(debug, "Input source detected: memory (byte array)");
        PyInputSource::ByteArray(input)
    }
    else if let Ok(path) = input_obj.extract::<String>() {
        PyInputSource::File(PathBuf::from(path))
    }
    else {
        PyInputSource::Reader(input)
    };

    // ---------- OUTPUT ----------

    let py_output = 
        if output_obj.is_instance_of::<PyBytes>()
        || output_obj.is_instance_of::<PyByteArray>()
        || output_obj.is_instance_of::<PyMemoryView>()
    {
        PyOutputSink::Memory
    }
    else if let Ok(path) = output_obj.extract::<String>() {
        PyOutputSink::File(PathBuf::from(path))
    }
    else {
        PyOutputSink::Writer(output)
    };

    Ok((py_input, py_output))
}

pub fn to_core_input<'py>(
    py: Python<'py>,
    src: PyInputSource,
    chunk_size: Option<usize>,
    debug: Option<bool>,
) -> PyResult<InputSource<'static>> {  // <-- use 'static with owned Vec
    match src {
        PyInputSource::Bytes(obj) => {
            let bytes = obj.bind(py).downcast::<PyBytes>()?.as_bytes().to_vec();
            dbg_detect!(debug, "Input source detected: memory (bytes)");
            Ok(InputSource::Reader(Box::new(std::io::Cursor::new(bytes))))
        }
        // ### 👉 FIXME: WITH:
        // ```rust
        // PyInputSource::Bytes(obj) => {
        //     let bytes = obj.bind(py).downcast::<PyBytes>()?.as_bytes();
        //     Ok(InputSource::Memory(bytes))
        // }
        // ```
        PyInputSource::ByteArray(obj) => {
            let bytes = unsafe {
                obj.bind(py).downcast::<PyByteArray>()?.as_bytes().to_vec()
            };
            dbg_detect!(debug, "Input source detected: memory (byte array)");
            Ok(InputSource::Reader(Box::new(std::io::Cursor::new(bytes))))
        }
        PyInputSource::File(p) => {
            dbg_detect!(debug, "Input source detected: file");
            Ok(InputSource::File(p))
        },
        PyInputSource::Reader(obj) => {
            let reader: Box<dyn Read + Send> =
                if let Ok(r) = PyReaderReadInto::new(obj.clone_ref(py), chunk_size) {
                    dbg_detect!(debug, "Input source detected: reader (read into)");
                    Box::new(r)
                } else if let Ok(r) = PyReaderHT::new(obj.clone_ref(py)) {
                    dbg_detect!(debug, "Input source detected: reader (read ht)");
                    Box::new(r)
                } else {
                    dbg_detect!(debug, "Input source detected: reader (generic)");
                    Box::new(PyReader::new(obj.clone_ref(py)))
                };
            Ok(InputSource::Reader(reader))
        }
    }
}

pub fn to_core_output(
    py: Python,
    sink: PyOutputSink,
    debug: Option<bool>,
) -> PyResult<OutputSink> {

    match sink {

        PyOutputSink::Memory => {
            dbg_detect!(debug, "Output source detected: memory (bytes/bytes array/memory view)");
            Ok(OutputSink::Memory)
        }

        PyOutputSink::File(p) => {
            dbg_detect!(debug, "Output source detected: file");
            Ok(OutputSink::File(p))
        }

        PyOutputSink::Writer(obj) => {

            let writer: Box<dyn Write + Send> =
                if let Ok(w) = PyWriterWriteInto::new(obj.clone_ref(py)) {
                    dbg_detect!(debug, "Output source detected: writer (write into)");
                    Box::new(w)
                }
                else if let Ok(w) = PyWriterHT::new(obj.clone_ref(py)) {
                    dbg_detect!(debug, "Output source detected: writer (write ht)");
                    Box::new(w)
                }
                else {
                    dbg_detect!(debug, "Output source detected: writer (generic)");
                    Box::new(PyWriter::new(obj.clone_ref(py)))
                };

            Ok(OutputSink::Writer(writer))
        }
    }
}

// - **InputSource::Memory** → `bytes`, `bytearray`, `memoryview`  
// - **InputSource::File** → `str` path  
// - **InputSource::Reader** → `BytesIO`, file‑like objects, custom readers  

// This way, our API behaves symmetrically:  
// - Passing `bytes`/`bytearray` gives us snapshot buffers.  
// - Passing `BytesIO` gives us streaming behavior.  
// pub fn py_extract_io(
//     py: Python,
//     input: PyObject,  // bytes, str path, or file-like
//     output: PyObject,
//     chunk_size: Option<usize>,
// ) -> PyResult<(InputSource, OutputSink)> {
//     // ------------------- INPUT -------------------
//     let input_src = classify_input(py, input, chunk_size, Some(true))?;

//     // ------------------- OUTPUT -------------------
//     let output_sink = classify_output(py, output, Some(true))?;

//     Ok((input_src, output_sink))
// }

// ### Key changes
// - **No generic `extract::<Vec<u8>>()` fallback** — that was too permissive and allowed `str` to be mis‑extracted as bytes.  
// - **Explicit `PyString` check** — ensures only actual Python strings become `File` paths.  
// - **Symmetry** — memory types are always memory, strings are always files, everything else is treated as file‑like.
