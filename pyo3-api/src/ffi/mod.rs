
// ## 📝 pyo3-api/src/ffi/mod.rs

use pyo3::prelude::*;

mod ffi_io;
mod ffi_api;

#[pymodule(name = "api")]
pub fn register_api(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {

    // Register public api
    let _ = ffi_api::register_api(py, m);

    Ok(())
}
