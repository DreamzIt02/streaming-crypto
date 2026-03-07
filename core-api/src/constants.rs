
/// "RSE1" = Rust Streaming Envelope v1, magic number for this envelope version.
/// - If the constant represents a **protocol magic field** (like `"RSE1"` in a header), use `[u8; 4]`. 
/// - That way the type itself enforces “exactly 4 bytes” and matches our struct field type (`[u8; 4]`).
pub const MAGIC_RSE1: [u8; 4] = *b"RSE1";
pub const HEADER_V1: u16      = 1;

/// Industry-standard master key lengths (AES-128, AES-192, AES-256)
pub const MASTER_KEY_LENGTHS: [usize; 3] = [16, 24, 32];

/// Defaults when Option<T> is None
pub const DEFAULT_CHUNK_SIZE: usize = 64 * 1024; // 64 KB
/// Industry-standard chunk sizes (in bytes) ✅
pub const ALLOWED_CHUNK_SIZES: &[usize] = &[
    16 * 1024,    // 16 KiB  - IoT/embedded, constrained memory
    32 * 1024,    // 32 KiB  - Mobile devices, network packets
    64 * 1024,    // 64 KiB  - Default (good balance) ✅ RECOMMENDED
    128 * 1024,   // 128 KiB - Desktop apps
    256 * 1024,   // 256 KiB - Server applications
    1024 * 1024,  // 1 MiB   - Bulk data processing
    2048 * 1024,  // 2 MiB   - Large file transfers
    4096 * 1024,  // 4 MiB   - High-throughput systems
];
/// Max chunk size sanity bound (32 MiB).
pub const MAX_CHUNK_SIZE: usize = 32 * 1024 * 1024;

// Basic sanity: minimum length and maybe a magic prefix
// require first 4 bytes to be a magic number
// - If the constant is more of a **prefix marker** we’ll check against slices (like `"DICT"` at the start of a dictionary payload), then `&[u8]` is fine:
pub const MAGIC_DICT: &[u8] = b"DICT";
pub const MIN_DICT_LEN: usize = 8;
pub const MAX_DICT_LEN: usize = 1 << 20; // 1 MiB cap for sanity

/// Strategy choices for encoder metadata (decoder may still parallelize).
pub mod strategy_ids {
    pub const AUTO: u16       = 0x0000;
    pub const SEQUENTIAL: u16 = 0x0001;
    pub const PARALLEL: u16   = 0x0002;
}

/// Cipher suite identifiers (mirrored in headers).
pub mod cipher_ids {
    pub const AES256_GCM: u16        = 0x0001;
    pub const CHACHA20_POLY1305: u16 = 0x0002;

    pub fn name(id: u16) -> &'static str {
        match id {
            AES256_GCM        => "AES256_GCM",
            CHACHA20_POLY1305 => "CHACHA20_POLY1305",
            _                 => "UNKNOWN",
        }
    }
}

/// HKDF PRF identifiers (mirrored in headers).
pub mod prf_ids {
    pub const SHA256: u16    = 0x0001;
    pub const SHA512: u16    = 0x0002;
    pub const SHA3_256: u16  = 0x0003;
    pub const SHA3_512: u16  = 0x0004;
    pub const BLAKE3K: u16   = 0x0005; // keyed BLAKE3 (avoid unless policy requires)
}

/// Algorithm profile bundles cipher + PRF combinations.
pub mod alg_profile_ids {
    pub const AES256_GCM_HKDF_SHA256: u16         = 0x0101;
    pub const AES256_GCM_HKDF_SHA512: u16         = 0x0102;
    pub const CHACHA20_POLY1305_HKDF_SHA256: u16  = 0x0201;
    pub const CHACHA20_POLY1305_HKDF_SHA512: u16  = 0x0202;
    pub const CHACHA20_POLY1305_HKDF_BLAKE3K: u16 = 0x0203;
}

/// AAD domain identifiers.
pub mod aad_domain_ids {
    pub const GENERIC: u16       = 0x0001;
    pub const FILE_ENVELOPE: u16 = 0x0002;
    pub const PIPE_ENVELOPE: u16 = 0x0003;
}

/// DIGEST ALG identifiers (mirrored in headers).
pub mod digest_ids {
    // Sha224   = 0x0001,
    pub const SHA256: u16       = 0x0002;
    // Sha384   = 0x0003,
    pub const SHA512: u16       = 0x0004;
    // Sha3_224 = 0x0101,
    pub const SHA3_256: u16     = 0x0102;
    // Sha3_384 = 0x0103,
    pub const SHA3_512: u16     = 0x0104;
    pub const BLAKE3K: u16      = 0x0201; // UN-KEYED Blake3
}

/// Flag bitmask for optional features and metadata presence.
pub mod flags {
    pub const HAS_TOTAL_LEN: u16    = 0x0001;
    pub const HAS_CRC32: u16        = 0x0002;
    pub const HAS_TERMINATOR: u16   = 0x0004;
    pub const HAS_FINAL_DIGEST: u16 = 0x0008;
    pub const DICT_USED: u16        = 0x0010;
    pub const AAD_STRICT: u16       = 0x0020;
}

// ### 📊 Comparison table

// | **Setting** | **Industry Standard** | **Rationale** |
// |-------------|------------------------|---------------|
// | **Queue cap** | **[4–16](guide://action?prefill=Tell%20me%20more%20about%3A%204%E2%80%9316)** | **[low latency, avoids memory bloat](guide://action?prefill=Tell%20me%20more%20about%3A%20low%20latency%2C%20avoids%20memory%20bloat)** |
// | **Workers** | **[match physical cores](guide://action?prefill=Tell%20me%20more%20about%3A%20match%20physical%20cores)** (usually 4–16) | **[CPU‑bound crypto tasks scale linearly](guide://action?prefill=Tell%20me%20more%20about%3A%20CPU%E2%80%91bound%20crypto%20tasks%20scale%20linearly)** |
// | **Beyond 16** | **[rarely beneficial](guide://action?prefill=Tell%20me%20more%20about%3A%20rarely%20beneficial)** | **[context switching overhead dominates](guide://action?prefill=Tell%20me%20more%20about%3A%20context%20switching%20overhead%20dominates)** |

// ### 🧩 Why worker count matters
// - **Match physical cores**: Each worker is CPU‑bound (AES, compression, HKDF). Running more workers than cores just adds context‑switch overhead.  
// - **Typical range**: 4–16 workers for server‑class CPUs; 2–8 for laptops.  
// - **Scaling**: Beyond 16 workers, diminishing returns set in unless we’re on a many‑core server (32+ cores).  
pub const WORKERS_COUNT: &[usize] = &[2, 4, 8, 16];

// ### 🧩 Why queue cap matters
// - **Small queue (2–16)**: Keeps latency low, avoids excessive buffering, and ensures back‑pressure works correctly.  
// - **Large queue (>32)**: Can cause memory bloat, uneven scheduling, and delayed error propagation. Most cryptographic pipelines (AES, VPNs, TLS offload) deliberately cap queues at small powers of two.  
// - **Industry practice**: VPN engines, GPU crypto libraries, and parallel AES implementations typically use **queue caps of 4–16**.
pub const QUEUE_CAPS: &[usize] = &[2, 4, 8, 16];
pub const DEFAULT_WORKERS: usize = 2;            // or num_cpus::get().saturation_sub(1)
pub const DEFAULT_QUEUE_CAP: usize = 4;          // or workers * 2
