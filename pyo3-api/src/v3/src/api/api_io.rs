use std::{io::{Read, Write}, path::PathBuf};

use pyo3::{Bound, PyAny, PyErr, PyObject, PyResult, Python, types::{PyAnyMethods, PyByteArray, PyBytes, PyBytesMethods, PyMemoryView}};

use crate::{PyOutputSink, PyReader, PyReaderHT, PyReaderReadInto, PyWriter, PyWriterHT, PyWriterWriteInto};
use core_api::{stream::{InputSource, OutputSink}};

// ============================================================
// IO classification — v3
//
// Fast paths (in priority order):
//   1. In-memory  → InputSource::Memory(&[u8])   zero-copy, all types
//   2. File/Path  → InputSource::File(PathBuf)   parallel pread
//   3. Reader     → InputSource::Reader(Box<..>) serial fallback (slow path)
//
// All in-memory Python types are zero-copy in v3.
// The caller owns their data. If they modify it mid-encryption
// they get wrong ciphertext. That is their problem, not ours.
// ============================================================

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

// ============================================================
// PyInputSource — extended for v3
// ============================================================

pub enum PyInputSource {
    // ── In-memory (all zero-copy in v3) ──────────────────────
    Bytes(PyObject),          // bytes          — immutable, always safe
    ByteArray(PyObject),      // bytearray      — mutable, caller's risk
    MemoryView(PyObject),     // memoryview     — any backing type
    Mmap(PyObject),           // mmap.mmap      — OS-pinned pages
    Array(PyObject),          // array.array    — buffer protocol
    Numpy(PyObject),          // numpy.ndarray  — contiguous uint8

    // ── File ─────────────────────────────────────────────────
    File(PathBuf),            // str / pathlib.Path / os.PathLike

    // ── Reader (slow path) ───────────────────────────────────
    Reader(PyObject),         // any object with .read()
}

// ============================================================
// classify_py_io — v3
//
// Detection order matters:
//   - memoryview before bytes/bytearray (it is_instance_of both
//     in some CPython versions depending on what it wraps)
//   - mmap before generic reader (mmap has .read() so it would
//     match Reader if we don't catch it first)
//   - numpy before generic reader (same reason)
//   - pathlib.Path / os.PathLike before Reader
//   - str path last of the file group
// ============================================================

pub fn classify_py_io(
    py: Python,
    input: PyObject,
    output: PyObject,
    debug: Option<bool>,
) -> PyResult<(PyInputSource, PyOutputSink)> {
    let input_obj = input.bind(py);
    let output_obj = output.bind(py);

    // ── INPUT ─────────────────────────────────────────────────

    let py_input = if input_obj.is_instance_of::<PyMemoryView>() {
        // Catch memoryview first — it wraps bytes, bytearray, mmap, numpy, etc.
        // We don't need to inspect the backing type; we extract &[u8] uniformly
        // via the buffer protocol. Zero-copy regardless of what it wraps.
        dbg_detect!(debug, "Input source detected: memory (memoryview)");
        PyInputSource::MemoryView(input)
    }
    else if input_obj.is_instance_of::<PyBytes>() {
        dbg_detect!(debug, "Input source detected: memory (bytes)");
        PyInputSource::Bytes(input)
    }
    else if input_obj.is_instance_of::<PyByteArray>() {
        dbg_detect!(debug, "Input source detected: memory (bytearray)");
        PyInputSource::ByteArray(input)
    }
    else if is_mmap(py, &input_obj) {
        // mmap.mmap has .read() so it would fall through to Reader without
        // this check. Catch it here and route to Memory (OS-pinned, zero-copy).
        dbg_detect!(debug, "Input source detected: memory (mmap)");
        PyInputSource::Mmap(input)
    }
    else if is_numpy_bytes(py, &input_obj) {
        // numpy.ndarray with dtype=uint8, C-contiguous. Buffer protocol gives
        // us a stable pointer. Catch before Reader since ndarray has .read()
        // if it is a file-like wrapper, but a raw ndarray does not — still
        // catch early to be explicit.
        dbg_detect!(debug, "Input source detected: memory (numpy)");
        PyInputSource::Numpy(input)
    }
    else if is_array_module(py, &input_obj) {
        // array.array — buffer protocol, zero-copy.
        dbg_detect!(debug, "Input source detected: memory (array.array)");
        PyInputSource::Array(input)
    }
    else if let Some(path) = extract_path(py, &input_obj) {
        // Handles: str, bytes-as-path, pathlib.Path, os.PathLike
        dbg_detect!(debug, "Input source detected: file");
        PyInputSource::File(path)
    }
    else {
        // Slow path: any object with .read() — file objects, io.BytesIO,
        // custom readers, etc.
        dbg_detect!(debug, "Input source detected: reader (slow path)");
        PyInputSource::Reader(input)
    };

    // ── OUTPUT ────────────────────────────────────────────────

    let py_output = if output_obj.is_instance_of::<PyMemoryView>()
        || output_obj.is_instance_of::<PyBytes>()
        || output_obj.is_instance_of::<PyByteArray>()
        || output_obj.is_instance_of::<PyMemoryView>()
    {
        dbg_detect!(debug, "Output sink detected: memory");
        PyOutputSink::Memory
    }
    else if let Some(path) = extract_path(py, &output_obj) {
        dbg_detect!(debug, "Output sink detected: file");
        PyOutputSink::File(path)
    }
    else {
        dbg_detect!(debug, "Output sink detected: writer (slow path)");
        PyOutputSink::Writer(output)
    };

    Ok((py_input, py_output))
}

// ============================================================
// Detection helpers
// ============================================================

/// Extracts a PathBuf from: str, bytes path, pathlib.Path, os.PathLike
fn extract_path(_py: Python, obj: &Bound<PyAny>) -> Option<PathBuf> {
    // str path
    if let Ok(s) = obj.extract::<String>() {
        return Some(PathBuf::from(s));
    }
    // pathlib.Path or any os.PathLike — call __fspath__() which returns str
    if let Ok(fspath) = obj.call_method0("__fspath__") {
        if let Ok(s) = fspath.extract::<String>() {
            return Some(PathBuf::from(s));
        }
    }
    // bytes path (b"/some/path")
    if let Ok(b) = obj.downcast::<PyBytes>() {
        if let Ok(s) = std::str::from_utf8(b.as_bytes()) {
            let p = PathBuf::from(s);
            // Only treat as path if it looks like one (contains a separator)
            // to avoid misclassifying raw bytes input as a file path.
            if p.is_absolute() || s.contains('/') || s.contains('\\') {
                return Some(p);
            }
        }
    }
    None
}

/// Checks if obj is a mmap.mmap instance
fn is_mmap(py: Python, obj: &Bound<PyAny>) -> bool {
    let Ok(mmap_mod) = py.import_bound("mmap") else { return false; };
    let Ok(mmap_cls) = mmap_mod.getattr("mmap") else { return false; };
    obj.is_instance(&mmap_cls).unwrap_or(false)
}

/// Checks if obj is a numpy.ndarray with dtype uint8 and C-contiguous layout
fn is_numpy_bytes(py: Python, obj: &Bound<PyAny>) -> bool {
    let Ok(np) = py.import_bound("numpy") else { return false; };
    let Ok(ndarray_cls) = np.getattr("ndarray") else { return false; };
    if !obj.is_instance(&ndarray_cls).unwrap_or(false) { return false; }
    // Check dtype is uint8
    let Ok(dtype) = obj.getattr("dtype") else { return false; };
    let Ok(kind) = dtype.getattr("kind") else { return false; };
    let Ok(kind_str) = kind.extract::<String>() else { return false; };
    if kind_str != "u" { return false; }
    // Check itemsize == 1 (uint8, not uint16/uint32)
    let Ok(itemsize) = dtype.getattr("itemsize") else { return false; };
    let Ok(sz) = itemsize.extract::<usize>() else { return false; };
    if sz != 1 { return false; }
    // Check C-contiguous (row-major, no gaps in memory)
    let Ok(flags) = obj.getattr("flags") else { return false; };
    let Ok(c_contig) = flags.get_item("C_CONTIGUOUS") else { return false; };
    c_contig.extract::<bool>().unwrap_or(false)
}

/// Checks if obj is an array.array instance
fn is_array_module(py: Python, obj: &Bound<PyAny>) -> bool {
    let Ok(arr_mod) = py.import_bound("array") else { return false; };
    let Ok(arr_cls) = arr_mod.getattr("array") else { return false; };
    obj.is_instance(&arr_cls).unwrap_or(false)
}

// ============================================================
// extract_buffer — unified zero-copy &[u8] from any memory type
//
// Called in the PyO3 entry point (py_encrypt_stream_v3, etc.)
// while the GIL is still held. The slice is valid as long as:
//   - The owning PyObject is alive in the caller's stack frame
//   - allow_threads() has not yet been called (GIL still held
//     when we extract, then we move the slice into the closure)
//
// All types go through the buffer protocol (PyBuffer<u8>).
// This handles bytes, bytearray, memoryview, mmap, array.array,
// and numpy.ndarray uniformly — no per-type branching needed.
// ============================================================

pub fn extract_buffer<'py>(
    py: Python<'py>,
    obj: &PyObject,
) -> PyResult<&'py [u8]> {
    let bound = obj.bind(py);
    let buf = pyo3::buffer::PyBuffer::<u8>::get_bound(bound)?;
    // is_c_contiguous() ensures the memory is a single flat region
    // with no gaps — required for a valid &[u8] slice.
    if !buf.is_c_contiguous() {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "Input buffer must be C-contiguous (no gaps in memory)"
        ));
    }
    // SAFETY: The buffer protocol guarantees this pointer is valid
    // while the GIL is held and the owning object is alive.
    // The caller must keep the PyObject alive across allow_threads().
    let slice = unsafe {
        std::slice::from_raw_parts(
            buf.buf_ptr() as *const u8,
            buf.len_bytes(),
        )
    };
    Ok(slice)
}

// ============================================================
// to_core_input — v3
//
// Memory and File variants are handled directly in the entry
// points (py_encrypt_stream_v3 / py_decrypt_stream_v3) using
// extract_buffer() above. They never reach to_core_input().
//
// to_core_input() in v3 is ONLY the Reader slow path.
// chunk_size is passed as None from v3 entry points — the
// pipeline reads crypto.base.segment_size itself.
// ============================================================

pub fn to_core_input<'py>(
    py: Python<'py>,
    src: PyInputSource,
    chunk_size: Option<usize>,
    debug: Option<bool>,
) -> PyResult<InputSource<'static>> {
    match src {
        PyInputSource::Reader(obj) => {
            let reader: Box<dyn Read + Send> =
                if let Ok(r) = PyReaderReadInto::new(obj.clone_ref(py), chunk_size) {
                    dbg_detect!(debug, "Input source detected: reader (read_into)");
                    Box::new(r)
                } else if let Ok(r) = PyReaderHT::new(obj.clone_ref(py)) {
                    dbg_detect!(debug, "Input source detected: reader (half-trip)");
                    Box::new(r)
                } else {
                    dbg_detect!(debug, "Input source detected: reader (generic)");
                    Box::new(PyReader::new(obj.clone_ref(py)))
                };
            Ok(InputSource::Reader(reader))
        }
        // Memory and File variants should never reach here in v3.
        // They are matched directly in the entry point before this
        // function is called. Guard anyway to catch misuse.
        _ => Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "to_core_input: memory and file inputs must be handled before this call"
        )),
    }
}

// ============================================================
// to_core_output — unchanged from v2, included for completeness
// ============================================================

pub fn to_core_output(
    py: Python,
    sink: PyOutputSink,
    debug: Option<bool>,
) -> PyResult<OutputSink> {
    match sink {
        PyOutputSink::Memory => {
            dbg_detect!(debug, "Output sink detected: memory");
            Ok(OutputSink::Memory)
        }
        PyOutputSink::File(p) => {
            dbg_detect!(debug, "Output sink detected: file");
            Ok(OutputSink::File(p))
        }
        PyOutputSink::Writer(obj) => {
            let writer: Box<dyn Write + Send> =
                if let Ok(w) = PyWriterWriteInto::new(obj.clone_ref(py)) {
                    dbg_detect!(debug, "Output sink detected: writer (write_into)");
                    Box::new(w)
                } else if let Ok(w) = PyWriterHT::new(obj.clone_ref(py)) {
                    dbg_detect!(debug, "Output sink detected: writer (half-trip)");
                    Box::new(w)
                } else {
                    dbg_detect!(debug, "Output sink detected: writer (generic)");
                    Box::new(PyWriter::new(obj.clone_ref(py)))
                };
            Ok(OutputSink::Writer(writer))
        }
    }
}

// ============================================================
// Updated entry point pattern (both encrypt and decrypt)
//
// All memory variants collapse to a single arm using
// extract_buffer(). The PyObject is kept alive in `_pin` so
// the slice remains valid across allow_threads().
// ============================================================

// In py_encrypt_stream_v3 / py_decrypt_stream_v3, replace the
// per-type match arms with:

/*
let result = match py_input {

    // ── All in-memory types → zero-copy ──────────────────────
    // extract_buffer() uses the buffer protocol uniformly across
    // bytes, bytearray, memoryview, mmap, array.array, numpy.
    // `_pin` keeps the PyObject alive (and thus the slice valid)
    // for the entire duration of allow_threads().
    PyInputSource::Bytes(ref obj)
    | PyInputSource::ByteArray(ref obj)
    | PyInputSource::MemoryView(ref obj)
    | PyInputSource::Mmap(ref obj)
    | PyInputSource::Array(ref obj)
    | PyInputSource::Numpy(ref obj) => {
        let slice = extract_buffer(py, obj)?;
        let _pin = obj;  // keeps PyObject alive across allow_threads()
        py.allow_threads(|| {
            encrypt_stream_v3(
                InputSource::Memory(slice),
                output_sink,
                &master_key,
                params,
                config,
            )
        })
    }

    // ── File → parallel pread ─────────────────────────────────
    PyInputSource::File(path) => {
        py.allow_threads(|| {
            encrypt_stream_v3(
                InputSource::File(path),
                output_sink,
                &master_key,
                params,
                config,
            )
        })
    }

    // ── Reader → slow path ────────────────────────────────────
    PyInputSource::Reader(_) => {
        let input_src = to_core_input(py, py_input, None, Some(true))?;
        py.allow_threads(|| {
            encrypt_stream_v3(
                input_src,
                output_sink,
                &master_key,
                params,
                config,
            )
        })
    }
};
*/
