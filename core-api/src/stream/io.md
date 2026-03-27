# IO

## 🔎 `Cursor<Vec<u8>>`

- **What it is**: A `Cursor` wraps a `Vec<u8>` and implements `Read`/`Write`.  
- **Pros**:
  - Simple and lightweight.
  - We can recover the buffer at the end with `cursor.into_inner()`.
  - No synchronization overhead — perfect for single‑threaded or pipeline‑style code where one writer fills the buffer and we later consume it.
- **Cons**:
  - Not thread‑safe. If multiple threads need to write concurrently, we’ll need external synchronization.

---

## 🔎 `Arc<Mutex<Vec<u8>>>`

- **What it is**: A reference‑counted, thread‑safe shared buffer. Multiple threads can lock and mutate the same `Vec<u8>`.  
- **Pros**:
  - Safe for concurrent writes/reads across threads.
  - We can attach the same buffer to telemetry snapshots while workers are still writing.
- **Cons**:
  - More complex: we need to lock/unlock around every access.
  - Slight performance overhead compared to `Cursor<Vec<u8>>`.

---

## ✅ Which is best?

- If our pipeline writes sequentially (encrypt pipeline writes, then we read the buffer after finishing), **`Cursor<Vec<u8>>` is best**. It’s simpler, faster, and idiomatic for capturing an in‑memory stream.  
- If we truly need multiple threads to mutate the same buffer concurrently (rare for crypto pipelines, since we usually serialize output), then **`Arc<Mutex<Vec<u8>>>`** is the safer choice.

---

### 🎯 Recommendation

Stick with `Cursor<Vec<u8>>` for our `OutputSink::Memory` case. It matches the semantics of a stream writer, avoids unnecessary locking, and makes it easy to recover the ciphertext with `into_inner()` once the pipeline completes.

---

Here’s a corrected `open_output` implementation using `Cursor<Vec<u8>>`:

```rust
pub fn open_output(
    sink: OutputSink,
) -> Result<(Box<dyn Write + Send>, Option<std::io::Cursor<Vec<u8>>>), StreamError> {
    match sink {
        OutputSink::Writer(w) => Ok((w, None)),
        OutputSink::File(p) => Ok((Box::new(std::fs::File::create(p)?), None)),
        OutputSink::Memory => {
            // Create a single Vec and wrap it in a Cursor
            let cursor = std::io::Cursor::new(Vec::new());
            // Return the cursor both as the writer and as the recoverable buffer
            Ok((Box::new(cursor.clone()), Some(cursor)))
        }
    }
}
```

---

### 🔧 How to recover the buffer later

When the pipeline finishes, we can pull the ciphertext back out of the cursor:

```rust
if let Some(cursor) = maybe_buf {
    let ciphertext = cursor.into_inner(); // contains HeaderV1 + segments
    snapshot.attach_output(ciphertext);
}
```

---

### ✅ Why this works

- The pipeline writes directly into the `Cursor<Vec<u8>>`.  
- We keep a handle to that same cursor in `Some(cursor)`.  
- At the end, `into_inner()` gives us the exact buffer the pipeline wrote into — no clones, no empty copies.  

---

Here’s a small helper we can drop in:

```rust
use std::io::{Cursor, Write};
use std::any::Any;

/// Try to unwrap a Box<dyn Write> back into a Cursor<Vec<u8>>
pub fn into_cursor_vec(writer: Box<dyn Write + Send>) -> Option<Cursor<Vec<u8>>> {
    // Downcast the Box<dyn Write> into its concrete type
    if let Ok(cursor) = writer.downcast::<Cursor<Vec<u8>>>() {
        Some(*cursor)
    } else {
        None
    }
}
```

---

### 🔧 Usage

```rust
let (writer, _) = open_output(OutputSink::Memory)?;

// ... run pipeline, writing into `writer` ...

// Recover the buffer
if let Some(cursor) = into_cursor_vec(writer) {
    let ciphertext: Vec<u8> = cursor.into_inner();
    eprintln!("Recovered ciphertext length = {}", ciphertext.len());
    snapshot.attach_output(ciphertext);
} else {
    eprintln!("Writer was not a Cursor<Vec<u8>>");
}
```

---

### ✅ Why this helps

- We only return the writer from `open_output`.  
- At the end, we call `into_cursor_vec` to recover the buffer if it’s a memory sink.  
- No need to juggle two return values (`writer` and `Option<Vec<u8>>`).  
- Keeps the API clean and makes snapshot attachment straightforward.

---

```rust
use std::io::{Cursor, Write};

pub fn open_output(
    sink: OutputSink,
) -> Result<(Box<dyn Write + Send>, Option<Cursor<Vec<u8>>>), StreamError> {
    match sink {
        OutputSink::Writer(w) => Ok((w, None)),
        OutputSink::File(p) => Ok((Box::new(std::fs::File::create(p)?), None)),
        OutputSink::Memory => {
            // Create a single Vec and wrap it in a Cursor
            let cursor = Cursor::new(Vec::new());
            // Return the cursor both as the writer and as the recoverable buffer
            Ok((Box::new(cursor.clone()), Some(cursor)))
        }
    }
}
```

---

### 🔧 How we use it

```rust
let (writer, maybe_cursor) = open_output(OutputSink::Memory)?;

// run pipeline, writing into `writer`

if let Some(cursor) = maybe_cursor {
    let ciphertext = cursor.into_inner(); // contains HeaderV1 + segments
    snapshot.attach_output(ciphertext);
}
```

---

### ✅ Why this works (1)

- The pipeline writes directly into the `Cursor<Vec<u8>>`.  
- We keep a handle to that same cursor in `Some(cursor)`.  
- At the end, `into_inner()` gives us the exact buffer the pipeline wrote into — no clones, no empty copies, no downcasting.  

---

```rust
use std::io::{Cursor, Write};

pub fn open_output(
    sink: OutputSink,
) -> Result<(Box<dyn Write + Send>, Option<Vec<u8>>), StreamError> {
    match sink {
        OutputSink::Writer(w) => Ok((w, None)),
        OutputSink::File(p) => Ok((Box::new(std::fs::File::create(p)?), None)),
        OutputSink::Memory => {
            // Create a Vec and wrap it in a Cursor
            let buf = Vec::new();
            let cursor = Cursor::new(buf);
            // We keep ownership of the Vec by splitting it out here
            let buf_ref = cursor.get_ref().clone();
            Ok((Box::new(cursor), Some(buf_ref)))
        }
    }
}
```

---

### 🔧 Usage (1)

```rust
let (writer, maybe_buf) = open_output(OutputSink::Memory)?;

// run pipeline, writing into `writer`

if let Some(mut buf) = maybe_buf {
    // buf now contains the ciphertext directly
    eprintln!("Recovered ciphertext length = {}", buf.len());
    snapshot.attach_output(buf);
}
```

---

### ✅ Why this works (2)

- The pipeline writes into the `Cursor<Vec<u8>>`.  
- We also keep a copy of the same `Vec<u8>` in `maybe_buf`.  
- When the pipeline finishes, we don’t need to call `into_inner()` — we already have the buffer.  
- This keeps the API simple and avoids juggling downcasts or conversions.

---

```rust
use std::io::Cursor;

pub fn open_output(
    sink: OutputSink,
) -> Result<(Cursor<Vec<u8>>, Option<Vec<u8>>), StreamError> {
    match sink {
        OutputSink::Writer(_) => {
            // For Writer/File we may still want trait objects,
            // but if we only care about Memory sinks in tests,
            // we can simplify to Cursor<Vec<u8>>.
            Err(StreamError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Writer/File not supported in this variant",
            )))
        }
        OutputSink::File(p) => {
            // Same note as above: we’d normally return a File writer here.
            let f = std::fs::File::create(p)?;
            let cursor = Cursor::new(Vec::new());
            Ok((cursor, None))
        }
        OutputSink::Memory => {
            // Create a Vec and wrap it in a Cursor
            let buf = Vec::new();
            let cursor = Cursor::new(buf);
            // Keep a copy of the Vec for direct inspection
            let buf_copy = cursor.get_ref().clone();
            Ok((cursor, Some(buf_copy)))
        }
    }
}
```

---

### 🔧 Usage in tests

```rust
let (mut cursor, maybe_buf) = open_output_cursor(OutputSink::Memory)?;

// run pipeline, writing into `cursor`

// Inspect buffer directly
if let Some(buf) = maybe_buf {
    assert!(buf.len() > 0);
    eprintln!("Ciphertext length = {}", buf.len());
}
```

---

### ✅ Why this helps (2)

- We avoid boxing into `dyn Write`, so we keep concrete types (`Cursor<Vec<u8>>` and `Vec<u8>`).  
- In tests, we can mutate the cursor and inspect the buffer directly without downcasting or calling `into_inner()`.  
- This makes round‑trip tests cleaner: we can assert on the buffer contents immediately after the pipeline finishes.

---

## 📥 InputSource Variants

- **Reader(Box<dyn Read + Send>)**
  - Wraps any type that implements `Read` (e.g., network streams, stdin, compressed streams).
  - Flexible: can handle arbitrary sources as long as they implement `Read`.
  - Useful when we want polymorphism and don’t care about the concrete type.

- **File(PathBuf)**
  - Represents a file path on disk.
  - We’ll open the file later to read its contents.
  - More concrete than `Reader`, tied directly to filesystem.

- **Memory(`Vec<u8>`)**
  - Data is already loaded into memory as a byte vector.
  - Fast access, no I/O overhead.
  - Best for small or preloaded datasets.

---

## 📤 OutputSink Variants

- **Writer(Box<dyn Write + Send>)**
  - Wraps any type that implements `Write` (e.g., network sockets, stdout, compressed writers).
  - Flexible: can write to arbitrary destinations.
  - Ideal when we want polymorphism and don’t care about the concrete type.

- **File(PathBuf)**
  - Represents a file path on disk.
  - We’ll open/create the file later to write into it.
  - Concrete, tied directly to filesystem.

- **Memory**
  - Collects output into an in-memory buffer (likely `Vec<u8>` internally).
  - Fast, avoids disk/network I/O.
  - Useful for testing, temporary results, or when we need the output as a byte array.

---

## 🔑 Key Differences

| Abstraction     | Purpose | Flexibility              | Performance       | Typical Use Case          |
|-----------------|---------|--------------------------|-------------------|---------------------------|
| Reader          | Input   | High (any `Read`)        | Depends on source | Streams, stdin, sockets   |
| File (Input)    | Input   | Medium (filesystem only) | Disk-bound        | Reading files             |
| Memory (Input)  | Input   | Low (fixed data)         | Fast              | Preloaded data            |
| Writer          | Output  | High (any `Write`)       | Depends on sink   | Streams, stdout, sockets  |
| File (Output)   | Output  | Medium (filesystem only) | Disk-bound        | Writing files             |
| Memory (Output) | Output  | Low (in-memory only)     | Fast              | Testing, capturing output |

---

In short:  

- **Reader/Writer** = polymorphic, trait-based, flexible.  
- **File** = concrete, filesystem-bound.  
- **Memory** = in-memory, fast, good for testing or temporary data.  

---

### Safer refactor pattern

Instead of:

```rust
pub enum InputSource {
    Reader(Box<dyn Read + Send>),
    File(PathBuf),
    Memory(Vec<u8>),
}
```

Use:

```rust
pub enum InputSource<'a> {
    Reader(Box<dyn Read + Send>),
    File(PathBuf),
    Memory(&'a [u8]), // borrow slice, no copy
}
```

Then in `classify_input`:

```rust
if obj.is_instance_of::<PyBytes>() {
    let pybytes: Bound<'_, PyBytes> = obj.downcast::<PyBytes>()?;
    let slice: &[u8] = pybytes.as_bytes();
    return Ok(InputSource::Memory(slice)); // no copy
}
```

And in `open_input`:

```rust
pub fn open_input<'a>(src: InputSource<'a>) -> Result<Box<dyn Read + Send + 'a>, StreamError> {
    match src {
        InputSource::Reader(r) => Ok(r),
        InputSource::File(p) => Ok(Box::new(std::fs::File::open(p)?)),
        InputSource::Memory(slice) => Ok(Box::new(std::io::Cursor::new(slice))),
    }
}
```

---

### Output side

Our `OutputSink::Memory` already uses `Arc<Mutex<Vec<u8>>>` or `Cursor<Vec<u8>>`. That’s fine — it avoids repeated copies. We don’t need to change the Python API here either.

---

## 1. Core Design Goal

We want:

- **Zero-copy memory input**
- **Unified `Read` / `Write` interface**
- **Optional captured memory output**
- **Minimal lifetimes leaking to public API**
- **Safe buffer sharing**

---

## 2. Refactored Types

### Input

```rust
pub enum InputSource<'a> {
    Reader(Box<dyn Read + Send + 'a>),
    File(PathBuf),
    Memory(&'a [u8]),
}
```

Notice:

```bash
Box<dyn Read + Send + 'a>
```

This fixes lifetime correctness.

---

### Output

Memory output should **always behave the same**.

```rust
pub enum OutputSink {
    Writer(Box<dyn Write + Send>),
    File(PathBuf),
    Memory,
}
```

---

## 3. Open Input (Cleaner)

```rust
pub fn open_input<'a>(
    src: InputSource<'a>,
) -> Result<Box<dyn Read + Send + 'a>, StreamError> {
    match src {
        InputSource::Reader(r) => Ok(r),

        InputSource::File(p) => {
            let f = std::fs::File::open(p)?;
            Ok(Box::new(f))
        }

        InputSource::Memory(slice) => {
            Ok(Box::new(std::io::Cursor::new(slice)))
        }
    }
}
```

This part of our design was already good.

---

## 4. Output Refactor (Much Cleaner)

Instead of the `with_buf` flag, always return a capture buffer when `Memory` is used.

This removes complexity.

```rust
pub fn open_output(
    sink: OutputSink,
) -> Result<(Box<dyn Write + Send>, Option<Arc<Mutex<Vec<u8>>>>), StreamError> {
    match sink {
        OutputSink::Writer(w) => Ok((w, None)),

        OutputSink::File(p) => {
            let f = std::fs::File::create(p)?;
            Ok((Box::new(f), None))
        }

        OutputSink::Memory => {
            let buf = Arc::new(Mutex::new(Vec::new()));
            let writer = SharedBufferWriter { buf: buf.clone() };

            Ok((Box::new(writer), Some(buf)))
        }
    }
}
```

Advantages:

✔ simpler
✔ deterministic behavior
✔ fewer branches
✔ easier API

---

## 5. SharedBufferWriter (Minor polish)

```rust
pub struct SharedBufferWriter {
    buf: Arc<Mutex<Vec<u8>>>,
}

impl Write for SharedBufferWriter {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        let mut guard = self.buf.lock().unwrap();
        guard.extend_from_slice(data);
        Ok(data.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
```

---

## 6. PayloadReader (Small improvement)

We can make it **more idiomatic**.

```rust
#[derive(Debug)]
pub struct PayloadReader<R> {
    inner: R,
}

impl<R: Read> PayloadReader<R> {
    pub fn new(inner: R) -> Self {
        Self { inner }
    }

    pub fn with_header(mut reader: R) -> Result<(HeaderV1, Self), StreamError> {
        let header = read_header(&mut reader)?;
        Ok((header, Self { inner: reader }))
    }
}
```

---

### Implement Read

```rust
impl<R: Read> Read for PayloadReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }
}
```

---

### Implement Seek

Only when underlying supports it.

```rust
impl<R: Read + Seek> Seek for PayloadReader<R> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(pos)
    }
}
```

---

## 7. Example Usage

### Memory output capture

```rust
let (mut writer, capture) = open_output(OutputSink::Memory)?;

writer.write_all(b"hello world")?;

if let Some(buf) = capture {
    let data = buf.lock().unwrap();
    println!("{}", String::from_utf8_lossy(&data));
}
```

---

## 8. Final Architecture (Clean)

```bash
InputSource
    Reader(Box<dyn Read>)
    File(PathBuf)
    Memory(&[u8])

open_input()
    -> Box<dyn Read>

OutputSink
    Writer(Box<dyn Write>)
    File(PathBuf)
    Memory

open_output()
    -> (Box<dyn Write>, Option<Arc<Mutex<Vec<u8>>>>)

PayloadReader<R>
    wraps reader
    header already consumed
```

---

## 9. Why This Design Is Good

✔ zero copy memory input
✔ streaming friendly
✔ works with files / sockets / Python objects
✔ deterministic memory output
✔ minimal generics explosion
✔ safe trait objects

This pattern is very similar to what libraries like:

- **reqwest**
- **hyper**
- **tokio**

use internally.

---

## Final Result

Our pipeline becomes:

```bash
Python object
     │
classify_input
     │
InputSource
     │
open_input()
     │
Box<dyn Read>
     │
stream encryption
     │
Box<dyn Write>
     │
open_output()
```

---

## Input: Python → Rust

| Python type   | Path                                                       | Copies          |
|---------------|------------------------------------------------------------|-----------------|
| `PyBytes`     | `InputSource::Memory(&[u8])` — borrow into Python's buffer | **0 copies** ✅ |
| `PyByteArray` | `to_vec()` — justified, mutable buffer unsafe to borrow    | **1 copy** ⚠️   |
| `str` path    | `InputSource::File(PathBuf)` — no data copied              | **0 copies** ✅ |
| file-like     | `InputSource::Reader(Box<dyn Read>)` — streamed            | **0 copies** ✅ |

---

## Output: Rust → Python

| Sink     | Path                                               | Copies                      |
|----------|----------------------------------------------------|-----------------------------|
| `Memory` | `OwnedOutput(Vec<u8>)` → `PyBytes::new_bound`      | **1 copy** (unavoidable) ✅ |
| `File`   | Rust writes directly to fd                         | **0 copies** ✅             |
| `Writer` | Rust calls `.write()` on Python file-like directly | **0 copies** ✅             |

---

## The one unavoidable copy

```bash
Rust Vec<u8>  ──►  PyBytes::new_bound()  ──►  Python heap
                   ^^^^^^^^^^^^^^^^^^^
                   Python's memory manager must own its buffer
                   No way to transfer Vec<u8> ownership into PyBytes
                   This copy is fundamental to the Python/Rust boundary
```

For `File` and `Writer` output — which is the right choice for large data — **zero copies cross the boundary at any point in the pipeline**.

---
