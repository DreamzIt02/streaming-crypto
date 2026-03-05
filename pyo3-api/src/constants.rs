use core_api::constants::{DEFAULT_CHUNK_SIZE, HEADER_V1, MAGIC_RSE1, MAX_CHUNK_SIZE, flags};
use pyo3::prelude::*;
use pyo3::types::PyModule;

#[pymodule(name = "constants")]
pub fn register_constants(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Protocol magic/version
    m.add("MAGIC_RSE1", MAGIC_RSE1.to_vec())?;
    m.add("HEADER_V1",  HEADER_V1)?;
    
    // Chunk sizes
    m.add("DEFAULT_CHUNK_SIZE", DEFAULT_CHUNK_SIZE)?;
    m.add("MAX_CHUNK_SIZE",     MAX_CHUNK_SIZE)?;

    // Submodule for flags
    let flags_mod = PyModule::new_bound(py, "flags")?;
    flags_mod.add("HAS_TOTAL_LEN",    flags::HAS_TOTAL_LEN)?;
    flags_mod.add("HAS_CRC32",        flags::HAS_CRC32)?;
    flags_mod.add("HAS_TERMINATOR",   flags::HAS_TERMINATOR)?;
    flags_mod.add("HAS_FINAL_DIGEST", flags::HAS_FINAL_DIGEST)?;
    flags_mod.add("DICT_USED",        flags::DICT_USED)?;
    flags_mod.add("AAD_STRICT",       flags::AAD_STRICT)?;
    m.add_submodule(&flags_mod)?;

    Ok(())
}
