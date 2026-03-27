# API V3

**Key observations:**

- `InputSource::Memory(&'scope [u8])` — lifetime tied to `crossbeam::scope`, so the slice just needs to outlive the scope. For PyO3, `PyBytes` is immutable + ref-counted → zero-copy safe with `py.allow_threads()` keeping the object alive. `PyByteArray` → **copy to `Vec<u8>` first**, then pass `InputSource::Memory(&bytes)` — same as v2, still the right call.
- `InputSource::File(PathBuf)` — straightforward, no chunk_size needed from PyO3 side (pipeline reads it from `crypto.base.segment_size`)
- `InputSource::Reader(Box<dyn Read + Send>)` — for Python file-like objects
- **No `to_core_input` boxing** — we construct `InputSource` directly in the PyO3 layer and pass it raw
- `decrypt_read_header` is called inside `decrypt_stream_v3` on the Rust side — PyO3 doesn't touch itHere's a summary of every decision made and why:

**What's identical to v2:** `PyEncryptParams`, `PyDecryptParams`, `PyApiConfig`, and their `From<>` impls are unchanged — the struct shapes didn't change in v3.

**What changed and why:**

| Concern                    | v2                                                         | v3                                   |
| -------------------------- | ---------------------------------------------------------- | ------------------------------------ |
| File path                  | `to_core_input()` opened the file and boxed it as `Reader` | `InputSource::File(path)` forwarded raw → pipeline opens with `Arc<File>` and spawns parallel `pread` workers |
| `chunk_size` hint          | Passed into `to_core_input()` for the Reader path          | Not needed at PyO3 layer — pipeline reads `crypto.base.segment_size` itself |
| Header stripping (decrypt) | `PayloadReader::with_header()` called in PyO3 layer        | `decrypt_read_header()` called inside `decrypt_stream_v3()` — all three variants handled in Rust |
| `PyByteArray`              | Copy to `Vec<u8>`                                          | Still copy — mutable buffer + parallel readers = data race, copy is the correct tradeoff |
| `PyBytes`                  | Zero-copy `&[u8]`                                          | Same — `obj` stays alive in scope, slice is valid across `allow_threads()` |

---

Here's a complete picture of every in-memory input Python can hand to PyO3, grouped by their safety properties:

**Immutable, pointer-stable (safe for zero-copy across `allow_threads`)**

`bytes` — the canonical immutable buffer in Python. Ref-counted, pointer never moves. `PyBytes::as_bytes()` gives a `&[u8]` that is valid as long as the `Py<PyBytes>` object is alive in scope. This is the ideal path.

`memoryview` of `bytes` — a view into a `bytes` object. Inherits immutability. The backing `bytes` object is kept alive by the view's internal reference. Same safety guarantee as raw `bytes`.

***Mutable but pointer-stable (zero-copy is safe if we hold the GIL-aware reference)**

`mmap.mmap` — memory-mapped file or anonymous mapping. The buffer is writable but the pointer is **OS-pinned** — the kernel will never move or reallocate it. Even across `allow_threads()`, the pointer remains valid. Python's `mmap` also implements the buffer protocol, so PyO3 can extract `&[u8]` via `PyBuffer<u8>`. The catch: another Python thread could write to it concurrently, so reads in Rust would see torn data in theory — but for encryption input this is the caller's problem, same as passing a file that's being written to.

`memoryview` of `mmap` — same stability guarantee, inherited from the mmap backing.

***Mutable and pointer-unstable (copy required)**

`bytearray` — resizable. CPython's internal buffer can be reallocated on any `.append()`, `.extend()`, `.clear()`, or slice assignment. Even a `memoryview` wrapping a `bytearray` only makes the *view* readonly — it raises `BufferError` if Python tries to resize while a view is active, but that protection only works while the GIL is held. Once we call `allow_threads()`, the GIL is released and that protection evaporates — another thread that already holds a reference can resize freely.

`memoryview` of `bytearray` — same underlying instability. The readonly flag on the view is a write guard, not a reallocation guard.

`array.array` — similar to `bytearray`, resizable, pointer-unstable.

***numpy arrays — special case**

`numpy.ndarray` — pointer is stable as long as the array is not resized or reallocated (i.e. no `resize()` or operations that change shape in-place). For a contiguous `uint8` array that the caller has no reason to resize during the call, it behaves like `mmap` in practice. But there's no formal guarantee unless the caller holds a reference and doesn't resize. PyO3 can access it via `PyBuffer<u8>`.

---

***Summary table**

| Python type                         | Pointer stable? | Immutable? | Zero-copy safe across `allow_threads`? |
|-------------------------------------|-----------------|------------|----------------------------------------|
| `bytes`                             | ✅              | ✅         | ✅                                     |
| `memoryview` of `bytes`             | ✅              | ✅         | ✅                                     |
| `mmap.mmap`                         | ✅ (OS-pinned)  | ❌         | ✅ (caller's risk for torn reads)      |
| `memoryview` of `mmap`              | ✅              | ❌         | ✅                                     |
| `bytearray`                         | ❌              | ❌         | ❌ copy required                       |
| `memoryview` of `bytearray`         | ❌              | ❌         | ❌ copy required                       |
| `array.array`                       | ❌              | ❌         | ❌ copy required                       |
| `numpy.ndarray` (uint8, contiguous) | ✅ in practice  | ❌         | ⚠️ safe if caller doesn't resize       |

---

***The real answer is simple**

If the user modifies their `bytearray` mid-encryption:

- The pipeline reads whatever bytes are there at the time
- The output is a valid encryption of whatever garbage they produced
- They decrypt it later and get garbage back
- **That is entirely their problem**

No crash. No hang. No memory safety issue in any practical sense. The pointer does not move during a normal `.extend()` or item assignment — CPython only reallocates on a resize that exceeds current capacity, and even that is an edge case that requires a very specific sequence of events to produce a use-after-free.

---

***What actually happens in the worst case**

The absolute worst realistic scenario: they resize `bytearray` past its allocated capacity mid-read, the old pointer is freed, Rust reads a few bytes from freed memory before the next chunk. CPython's allocator is unlikely to have already reused that memory in the microseconds between free and the next read. In practice it reads stale-but-valid bytes and produces wrong ciphertext.

The user gets wrong output. They do not get a crashed process in any realistic scenario.

---

**So the correct approach for `bytearray` is**

Zero-copy. No copy. Trust the caller. Document it:

```bash
Note: passing a bytearray that is concurrently modified
produces undefined ciphertext. This is the caller's responsibility.
```

Same contract every C extension in the Python ecosystem uses. Same contract NumPy uses. Same contract the Python `struct` module uses.

---

***What we should actually copy and why**

Nothing. All in-memory inputs — `bytes`, `bytearray`, `memoryview`, `mmap` — go through as `InputSource::Memory(&[u8])` zero-copy. The user owns their data. We read it, encrypt it, hand the result back. What they do with their own memory is not our concern.

---
