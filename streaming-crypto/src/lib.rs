// ## 📝 streaming-crypto/src/lib.rs

// --- MODULES PUBLISH START ---
// (empty in dev mode, because core-api/ffi-api/pyo3-api are separate crates)

// /// Encrypts data by XORing each byte with 0xAA.
// ///
// /// # Examples
// ///
// /// ```
// /// use core_api::encrypt;
// ///
// /// let data = vec![1, 2, 3];
// /// let encrypted = encrypt(&data);
// /// assert_eq!(encrypted[0], 1 ^ 0xAA);
// /// ```
// #[cfg(feature = "core-api")]
// pub use core_api::encrypt; // re-export the FFI wrapper

// /// FFI wrapper for encryption.
// ///
// /// # Safety
// /// Returns a raw pointer. Caller must manage memory.
// ///
// /// FFI wrapper for encryption.
// ///
// /// # Safety
// /// This function returns a raw pointer. The caller must manage memory.
// ///
// /// # Examples
// ///
// /// ```
// /// use std::slice;
// /// use ffi_api::encrypt;
// ///
// /// let data = vec![1, 2, 3];
// /// let ptr = encrypt(data.as_ptr(), data.len());
// /// let encrypted = unsafe { slice::from_raw_parts(ptr, data.len()) };
// /// assert_eq!(encrypted[0], 1 ^ 0xAA);
// /// ```
// #[cfg(feature = "ffi-api")]
// pub use ffi_api::encrypt; // re-export the FFI wrapper

// /// # Examples
// ///
// /// ```
// /// use pyo3::prelude::*;
// /// use pyo3_api::encrypt;
// /// use pyo3::types::PyBytes;
// ///
// /// Python::with_gil(|py| {
// ///     let data = PyBytes::new_bound(py, &[1, 2, 3]);
// ///     
// ///     let encrypted = encrypt(py, &data).unwrap();
// /// 
// ///     assert_eq!(encrypted[0], 1 ^ 0xAA);
// ///     assert_eq!(encrypted[1], 2 ^ 0xAA);
// ///     assert_eq!(encrypted[2], 3 ^ 0xAA);
// /// });
// /// ```
// #[cfg(feature = "pyo3-api")]
// pub use pyo3_api::encrypt; // re-export the PyO3 wrapper

// #[cfg(feature = "pyo3-api")]
// pub use pyo3_api::streaming_crypto; // re-export the #[pymodule]
// --- MODULES PUBLISH END ---
