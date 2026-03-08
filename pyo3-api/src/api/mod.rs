
// ## 📝 pyo3-api/src/ffi/mod.rs

use pyo3::prelude::*;

mod api_io;
mod api;

pub use api::*;
#[pymodule(name = "api")]
pub fn register_api(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {

    // Register public api
    let _ = api::register_api(py, m);

    Ok(())
}
