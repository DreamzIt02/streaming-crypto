use criterion::{criterion_group, criterion_main, Criterion};
#[cfg(feature = "benchmarks")]
use core_api::{
    benchmarks::{bench_utils::{random_bytes}}, 
    compression::CompressionCodec, parallelism::ParallelismConfig
};
#[cfg(feature = "benchmarks")]
use core_v3::{
    benchmarks::{bench_v3_encrypt_memory::bench_v3_encrypt_memory_2_memory_sync,}
};

fn bench_memory(c: &mut Criterion) {
    #[cfg(feature = "benchmarks")]
    c.bench_function("bench_v3_memory_2_memory_sync", |b| {
        b.iter(|| {
            let payload = random_bytes(100 * 1024 * 1024);
            let chunk_size = 2 * 1024 * 1024;
            bench_v3_encrypt_memory_2_memory_sync(
                payload, // plaintext
                chunk_size,        // chunk size
                CompressionCodec::Auto,
                ParallelismConfig::new(4, 0, 0.5, 64),
            )
        });
    });
}

criterion_group!(benches, bench_memory);
criterion_main!(benches);
