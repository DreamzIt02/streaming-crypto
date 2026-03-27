
// ## 📝 pyo3-api/src/api/mod.rs

// use pyo3::prelude::*;

// mod api_io;
mod api;

// pub use api::*;

// pub use api_io::{
//     classify_py_io,
//     to_core_input,
//     to_core_output,
// };

// #[pymodule(name = "api")]
// pub fn register_api(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
//     // Register class
//     m.add_class::<PyEncryptParams>()?;
//     m.add_class::<PyDecryptParams>()?;
//     m.add_class::<PyApiConfig>()?;

//     // Register func
//     m.add_function(wrap_pyfunction!(py_encrypt_stream_v2, m)?)?;
//     m.add_function(wrap_pyfunction!(py_decrypt_stream_v2, m)?)?;

//     Ok(())
// }
