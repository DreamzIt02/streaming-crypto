// ## 📝 pyo3-api/src/parallelism.rs

use core_api::parallelism::ParallelismConfig;
use pyo3::{prelude::*, types::PyDict};

#[pyclass(name = "ParallelismConfig")]
#[derive(Debug, Clone)]
pub struct PyParallelismConfig {
    #[pyo3(get, set)]
    pub cpu_workers: usize,
    #[pyo3(get, set)]
    pub gpu_workers: usize,
    #[pyo3(get, set)]
    pub mem_fraction: f64,
    #[pyo3(get, set)]
    pub hard_cap: usize,
}

#[pymethods]
impl PyParallelismConfig {
    #[new]
    fn new(cpu_workers: usize, gpu_workers: usize, mem_fraction: f64, hard_cap: usize) -> Self {
        Self { cpu_workers, gpu_workers, mem_fraction, hard_cap }
    }

    fn as_dict(&self, py: Python<'_>) -> Py<PyDict> {
        let dict = PyDict::new_bound(py); // ✅ PyO3 0.22 style
        dict.set_item("cpu_workers", self.cpu_workers).unwrap();
        dict.set_item("gpu_workers", self.gpu_workers).unwrap();
        dict.set_item("mem_fraction", self.mem_fraction).unwrap();
        dict.set_item("hard_cap", self.hard_cap).unwrap();
        dict.unbind() // convert Bound<'_, PyDict> → Py<PyDict>
    }
}


// Conversion from core Rust struct to PyO3 wrapper
impl From<PyParallelismConfig> for ParallelismConfig {
    fn from(c: PyParallelismConfig) -> Self {
        ParallelismConfig::new(c.cpu_workers, c.gpu_workers, c.mem_fraction, c.hard_cap)
    }
}

#[pymodule(name = "parallelism")]
pub fn register_parallelism(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Register class
    m.add_class::<PyParallelismConfig>()?;
    Ok(())
}