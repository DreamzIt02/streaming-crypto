// bench_v3_decrypt_memory.rs

use std::time::Instant;
use std::path::PathBuf;

use crate::{
    benchmarks::{bench_results::{BenchmarkResult, make_result}, 
    bench_utils::{dummy_master_key, measure_memory_mb}}, 
    compression::CompressionCodec, parallelism::ParallelismConfig, 
    stream::{InputSource, OutputSink},
    utils::enum_name_or_hex
};

use crate::v3::stream::core::{ApiConfig, DecryptParams, decrypt_stream_v3};

// ### 📥 Memory Input Decrypt Macro
macro_rules! bench_v3_decrypt_memory {
    ($out_variant:ident, $out_expr:expr) => {
        paste::paste! {
            pub fn [<bench_v3_decrypt_memory_2_ $out_variant:lower _sync>](
                ciphertext: Vec<u8>,                // <-- always feed ciphertext buffer
                chunk_size: usize,
                compression: CompressionCodec,
                parallelism: ParallelismConfig,
            ) -> (BenchmarkResult, Option<Vec<u8>>, Option<PathBuf>) {
                let comp_name = &enum_name_or_hex::<CompressionCodec>(compression as u16);
                let scenario = &format!("memory_2_{}_{}_sync",
                    stringify!($out_variant).to_lowercase(),
                    comp_name
                );

                let master_key = dummy_master_key();
                let params_dec = DecryptParams { master_key: master_key };
                let api_config = ApiConfig::new(Some(true), None, None, Some(parallelism));

                let mem_before = measure_memory_mb();
                let start = Instant::now();
                let snapshot_dec = decrypt_stream_v3(
                    InputSource::Memory(&ciphertext),   // <-- always memory input
                    $out_expr,
                    params_dec,
                    api_config,
                ).unwrap();

                let plaintext_size = snapshot_dec.output
                    .as_ref()
                    .map(|v| v.0.len())
                    .unwrap_or_else(|| {
                        // fallback: use ciphertext length or header info
                        ciphertext.len() // if ciphertext buffer is available
                    });

                let result = make_result(
                    scenario, "decrypt", "sync",
                    plaintext_size,
                    comp_name, chunk_size, start, mem_before,
                    snapshot_dec.output.as_ref().map(|v| v.0.len()), None
                );

                match &$out_expr {
                    OutputSink::Memory => {
                        // Consume snapshot, split into (snapshot_without_output, output_bytes)
                        let (_, output_bytes) = snapshot_dec.take_output();
                        (result, output_bytes, None)
                    }
                    OutputSink::File(path) => {
                        // File sink never carries output buffer
                        (result, None, Some(path.clone()))
                    }
                    OutputSink::Writer(_) => {
                        // Same as Memory: consume and extract buffer
                        let (_, output_bytes) = snapshot_dec.take_output();
                        (result, output_bytes, None)
                    }
                }
            }
        }
    };
}

// ### 📥 Memory Input Macro Calls

// Decrypt from Memory → Memory
bench_v3_decrypt_memory!(Memory, OutputSink::Memory);

// Decrypt from Memory → File
bench_v3_decrypt_memory!(File, OutputSink::File(PathBuf::from("decrypted_memory.dat")));

// Decrypt from Memory → Writer
bench_v3_decrypt_memory!(Writer, OutputSink::Writer(Box::new(Vec::new())));

// they expand into three functions:

// - `bench_v3_decrypt_memory_2_memory_sync`
// - `bench_v3_decrypt_memory_2_file_sync`
// - `bench_v3_decrypt_memory_2_writer_sync`

// ### 📥 Usage Examples

// ```rust
// fn main() {
//     let payload_size = 1024 * 1024; // 1 MB
//     let chunk_size = 4096;
//     let compression = CompressionCodec::None;
//     let parallelism = ParallelismConfig::default();

//     // Suppose we already ran an encrypt function:
//     let (enc_result, ciphertext, _) = bench_v3_encrypt_memory_2_memory_sync(
//         payload_size, chunk_size, compression, parallelism
//     );

//     // Decrypt from Memory → Memory
//     let (dec_result_mem, plaintext_mem, _) = bench_v3_decrypt_memory_2_memory_sync(
//         ciphertext.clone().unwrap(), // feed ciphertext buffer
//         chunk_size, compression, parallelism
//     );
//     println!("Decrypt Memory→Memory result: {:?}", dec_result_mem);
//     println!("Plaintext length: {:?}", plaintext_mem.as_ref().map(|v| v.len()));

//     // Decrypt from Memory → File
//     let (dec_result_file, _, file_path) = bench_v3_decrypt_memory_2_file_sync(
//         ciphertext.clone().unwrap(),
//         chunk_size, compression, parallelism
//     );
//     println!("Decrypt Memory→File result: {:?}", dec_result_file);
//     println!("Plaintext written to: {:?}", file_path);

//     // Decrypt from Memory → Writer
//     let (dec_result_writer, plaintext_writer, _) = bench_v3_decrypt_memory_2_writer_sync(
//         ciphertext.unwrap(),
//         chunk_size, compression, parallelism
//     );
//     println!("Decrypt Memory→Writer result: {:?}", dec_result_writer);
//     println!("Plaintext length: {:?}", plaintext_writer.as_ref().map(|v| v.len()));
// }
// ```

// ### 🔑 Key Points
// - We always pass the ciphertext buffer (`Vec<u8>`) from the encrypt macro into the decrypt macro.  
// - The decrypt macro then writes the plaintext into the chosen sink (`Memory`, `File`, or `Writer`).  
// - The return tuple gives we the benchmark result plus either the plaintext buffer or the file path, depending on the sink.  
