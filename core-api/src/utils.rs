use std::fmt;
use std::sync::Once;
use num_enum::TryFromPrimitive;
use tracing::Level;
use tracing_subscriber::EnvFilter;

pub fn enum_name_or_hex<T>(raw: T::Primitive) -> String
where
    T: TryFromPrimitive + fmt::Debug,
    T::Primitive: fmt::LowerHex,
{
    match T::try_from_primitive(raw) {
        Ok(variant) => format!("{:?}", variant),
        Err(_) => format!("0x{:x}", raw),
    }
}

pub fn fmt_bytes(b: &[u8]) -> String {
    if b.iter().all(|&c| c.is_ascii_graphic() || c == b' ') {
        format!("b\"{}\"", String::from_utf8_lossy(b))
    } else {
        format!("0x{}", hex::encode(b))
    }
}

pub fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect::<String>()
}

#[repr(u16)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, TryFromPrimitive)]
pub enum ChecksumAlg {
    Crc32   = 0x0001,
    Blake3   = 0x0201, // UN-KEYED Blake3
}
pub fn compute_checksum(data: &[u8], alg: Option<ChecksumAlg>) -> u32 {
    match alg {
        Some(ChecksumAlg::Crc32)  => compute_crc32(data),
        // Some(ChecksumAlg::Blake3) => compute_blake3(data), // Its return 32-bytes
        _                         => compute_crc32(data)
    }
}

fn compute_crc32(data: &[u8]) -> u32 {
    use crc32fast::Hasher;
    let mut hasher = Hasher::new();
    hasher.update(data);
    hasher.finalize()
}

// fn compute_blake3(data: &[u8]) -> [u8; 32] {
//     use blake3::Hasher;
//     let mut hasher = Hasher::new();
//     hasher.update(data);
//     *hasher.finalize().as_bytes()
// }

static INIT: Once = Once::new();
/// Initialize a default tracing logger with optional level.
/// Call this once at the start of our program or benchmark.
pub fn tracing_logger(_level: Option<Level>) {
    INIT.call_once(|| {
        // Build filter from RUST_LOG or fallback to provided level 
        let filter = EnvFilter::from_default_env(); // respects RUST_LOG
            // .add_directive(level.unwrap_or(Level::INFO).into());

        let subscriber = tracing_subscriber::fmt()
            .with_env_filter(filter)
            // .with_max_level(level.unwrap_or(Level::INFO)) // default to INFO if None
            .with_target(false)            // hide module path
            .with_thread_names(true)       // show thread names
            .finish();

        tracing::subscriber::set_global_default(subscriber)
            .expect("Failed to set global tracing subscriber");
    });
}

// ```rust
// fn main() {
//     // Default INFO level
//     tracing_logger(None);

//     // Or explicitly set DEBUG level
//     tracing_logger(Some(tracing::Level::DEBUG));

//     tracing::info!("Benchmark starting");
//     tracing::debug!("Debug details: {:?}", 42);
// }
// ```

// ### 🔑 Notes
// - `tracing_logger(None)` → defaults to `INFO`.  
// - `tracing_logger(Some(Level::DEBUG))` → enables debug output.  
// - We can still override with environment variables (`RUST_LOG=error cargo bench`) if we add `.with_env_filter(...)` to the subscriber.  
