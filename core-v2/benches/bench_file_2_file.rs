use criterion::{criterion_group, criterion_main, Criterion};
#[cfg(feature = "benchmarks")]
use core_api::{
    benchmarks::{
        bench_utils::random_bytes,
    },
    compression::CompressionCodec,
    parallelism::ParallelismConfig,
};

#[cfg(feature = "benchmarks")]
use core_v2::benchmarks::{
    bench_v2_encrypt_file::bench_v2_encrypt_file_2_file_sync,
};

#[cfg(feature = "benchmarks")]
use std::{fs::File, io::Write, path::PathBuf};

fn bench_file(c: &mut Criterion) {
    #[cfg(feature = "benchmarks")]
    c.bench_function("bench_v2_file_2_file_sync", |b| {
        b.iter(|| {
            // Generate random payload
            let payload = random_bytes(100 * 1024 * 1024);

            // Write payload to a temporary file
            let tmp_path = PathBuf::from("bench_input.dat");
            let mut f = File::create(&tmp_path).expect("Failed to create temp file");
            f.write_all(&payload).expect("Failed to write payload");

            // Benchmark encryption from File → File
            let chunk_size = 2 * 1024 * 1024;
            bench_v2_encrypt_file_2_file_sync(
                tmp_path.clone(), // pass file path, not raw bytes
                chunk_size,
                CompressionCodec::Auto,
                ParallelismConfig::new(4, 0, 0.5, 64),
            )
        });
    });
}

criterion_group!(benches, bench_file);
criterion_main!(benches);
