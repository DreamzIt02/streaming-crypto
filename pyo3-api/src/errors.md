# Errors

## 🎯 What Is Actually Happening (gil-refs)

In **PyO3 0.22**, the `create_exception!` macro internally expands to code that still references:

```rust
#[cfg(feature = "gil-refs")]
```

With Rust 1.92:

* Rust now checks all `cfg(feature = "...")`
* It expects the feature to exist in *our crate*
* `gil-refs` is not defined in our crate
* So it emits:

```bash
unexpected `cfg` condition value: `gil-refs`
```

This is a **lint warning**, not an error.

our code is correct.

---

## 🏗 Since We’re Doing CI Matrix Builds

Add it once in our PyO3 wrapper crate (`pyo3-api/src/lib.rs`) and:

```rust
#![allow(unexpected_cfgs)]
```

* Linux build ✅
* macOS build ✅
* Windows build ✅
* GitHub Actions stable toolchain ✅

---

If we want zero warnings across all crates:

In workspace `Cargo.toml`:

```toml
[lints.rust]
unexpected_cfgs = "allow"
```

---

## 1️⃣ Version A — Simple `args` only

```rust
impl From<PyStreamError> for PyErr {
    fn from(err: PyStreamError) -> PyErr {
        // Python users can do:
        // code, message = e.args
        StreamError::new_err((err.code.clone(), err.message.clone()))
        // Python users can't do:
        // e.code
        // e.message
    }
}
```

* Allocates a **Python exception object** (`StreamError`) with `args = (code, message)`
* `err.code.clone()` and `err.message.clone()` → **Rust `String` clone** (cheap, predictable)
* Returns `PyErr` immediately
* No GIL re-entry needed beyond `new_err` (PyO3 automatically handles it)

## 2️⃣ Version B — Attach `e.code` / `e.message`

```rust
Python::with_gil(|py| {
    let exc = StreamError::new_err((err.code.clone(), err.message.clone()));
    let obj = exc.clone_ref(py).into_value(py);
    obj.setattr("code", err.code.clone()).ok();
    obj.setattr("message", err.message.clone()).ok();
    exc
})
```

1. **Acquire GIL** explicitly with `Python::with_gil(|py| ...)`
2. Call `StreamError::new_err` → allocates exception + `args` tuple
3. `clone_ref(py)` → increases PyO3 reference count (extra atomic increment)
4. `into_value(py)` → gets the actual Python object
5. `setattr` twice → two **Python C API calls**, each with **GIL checks** + possibly **dictionary mutation**
6. Return `PyErr`

## 3️⃣ Summary Comparison (Performance)

| Aspect                      | Version A (args only) | Version B (args + code/message) |
| --------------------------- | --------------------- | ------------------------------- |
| Rust `String` clone         | 2                     | 2                               |
| Python exception allocation | 1                     | 1                               |
| Python `setattr` calls      | 0                     | 2                               |
| GIL manipulation            | minimal               | explicit via `with_gil`         |
| Hot path performance        | best                  | slightly slower                 |
| Python usability (`e.code`) | ❌                    | ✅                              |
| Python usability (`e.args`) | ✅                    | ✅                              |

---

### ✅ Practical Option

* **If exceptions are rare** (like crypto errors, validation failures): use Version B. The extra `setattr` cost is tiny.

* **If exceptions are frequent in hot loops** (millions of times): stick with Version A and use `e.args` for structured data — no extra Python API calls.

* **Hybrid Option:** Version B is fine for our crypto library — exceptions are rare, and Python users get convenient `.code` and `.message`.

---
