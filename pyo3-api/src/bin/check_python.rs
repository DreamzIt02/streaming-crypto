use pyo3::prelude::*;
use pyo3::types::PyModule;

fn main() -> PyResult<()> {
    Python::with_gil(|py| {
        // Use import_bound if import is not available
        let sys = PyModule::import_bound(py, "sys")?;
        let version: String = sys.getattr("version")?.extract()?;
        let prefix: String = sys.getattr("prefix")?.extract()?;
        println!("Python version: {}", version);
        println!("Python prefix: {}", prefix);
        Ok(())
    })
}