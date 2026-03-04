// ## 📝 pyo3-api/src/telemetry.rs

// * ✅ Full Rust → Python mapping
// * ✅ All fields exposed
// * ✅ Proper `Duration` conversion
// * ✅ `StageTimes` exposed cleanly
// * ✅ Proper `__repr__`
// * ✅ Proper `.pyi` stub
// * ✅ Production quality

// Assumptions:

// * `StageTimes` = `HashMap<Stage, Duration>`
// * `Stage` implements `ToString` or `Display`

// We expose:

use core_api::telemetry::TelemetrySnapshot;
use pyo3::{prelude::*, types::{PyBytes, PyDict}};

// * `elapsed` → seconds (f64)
// * `stage_times` → `Dict[str, float]`
// * `output` → `bytes | None`
#[pyclass(name = "TelemetrySnapshot")]
pub struct PyTelemetrySnapshot {
    #[pyo3(get)]
    pub segments_processed: u64,

    #[pyo3(get)]
    pub frames_data: u64,

    #[pyo3(get)]
    pub frames_terminator: u64,

    #[pyo3(get)]
    pub frames_digest: u64,

    #[pyo3(get)]
    pub bytes_plaintext: u64,

    #[pyo3(get)]
    pub bytes_compressed: u64,

    #[pyo3(get)]
    pub bytes_ciphertext: u64,

    #[pyo3(get)]
    pub bytes_overhead: u64,

    #[pyo3(get)]
    pub compression_ratio: f64,

    #[pyo3(get)]
    pub throughput_plaintext_bytes_per_sec: f64,

    /// elapsed seconds (Duration converted)
    #[pyo3(get)]
    pub elapsed_sec: f64,

    /// stage_times as Python dict[str, float]
    #[pyo3(get)]
    pub stage_times: PyObject,

    #[pyo3(get)]
    pub output: Option<Py<PyBytes>>,
}

impl From<TelemetrySnapshot> for PyTelemetrySnapshot {
    fn from(snap: TelemetrySnapshot) -> Self {
        Python::with_gil(|py| {
            // Convert stage times -> Dict[str, float]
            let stage_dict = PyDict::new_bound(py);

            for (stage, duration) in snap.stage_times.iter() {
                let seconds = duration.as_secs_f64();
                stage_dict
                    .set_item(stage.to_string(), seconds)
                    .expect("stage_times insert failed");
            }

            let py_output = snap.output.map(|v| {
                PyBytes::new_bound(py, &v).into()
            });

            Self {
                segments_processed: snap.segments_processed,
                frames_data: snap.frames_data,
                frames_terminator: snap.frames_terminator,
                frames_digest: snap.frames_digest,
                bytes_plaintext: snap.bytes_plaintext,
                bytes_compressed: snap.bytes_compressed,
                bytes_ciphertext: snap.bytes_ciphertext,
                bytes_overhead: snap.bytes_overhead,
                compression_ratio: snap.compression_ratio,
                throughput_plaintext_bytes_per_sec: snap.throughput_plaintext_bytes_per_sec,
                elapsed_sec: snap.elapsed.as_secs_f64(),
                stage_times: stage_dict.into(),
                output: py_output,
            }
        })
    }
}

#[pymethods]
impl PyTelemetrySnapshot {
    fn __repr__(&self, py: Python) -> PyResult<String> {
        let output_len = match &self.output {
            Some(obj) => obj.bind(py).len()?,
            None => 0,
        };

        Ok(format!(
            "PyTelemetrySnapshot(segments_processed={}, bytes_plaintext={}, bytes_ciphertext={}, compression_ratio={:.6}, throughput={:.2} B/s, elapsed_sec={:.6}, output_len={})",
            self.segments_processed,
            self.bytes_plaintext,
            self.bytes_ciphertext,
            self.compression_ratio,
            self.throughput_plaintext_bytes_per_sec,
            self.elapsed_sec,
            output_len
        ))
    }

    fn __str__(&self, py: Python) -> PyResult<String> {
        self.__repr__(py)
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);

        dict.set_item("segments_processed", self.segments_processed)?;
        dict.set_item("frames_data", self.frames_data)?;
        dict.set_item("frames_terminator", self.frames_terminator)?;
        dict.set_item("frames_digest", self.frames_digest)?;
        dict.set_item("bytes_plaintext", self.bytes_plaintext)?;
        dict.set_item("bytes_compressed", self.bytes_compressed)?;
        dict.set_item("bytes_ciphertext", self.bytes_ciphertext)?;
        dict.set_item("bytes_overhead", self.bytes_overhead)?;
        dict.set_item("compression_ratio", self.compression_ratio)?;
        dict.set_item(
            "throughput_plaintext_bytes_per_sec",
            self.throughput_plaintext_bytes_per_sec,
        )?;
        dict.set_item("elapsed_sec", self.elapsed_sec)?;
        dict.set_item("stage_times", &self.stage_times)?;

        if let Some(ref output) = self.output {
            dict.set_item("output_len", output.bind(py).len()?)?;
        } else {
            dict.set_item("output_len", 0)?;
        }

        Ok(dict.into())
    }
}

#[pymodule(name = "telemetry")]
pub fn register_telemetry(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Register class
    m.add_class::<PyTelemetrySnapshot>()?;
    Ok(())
}