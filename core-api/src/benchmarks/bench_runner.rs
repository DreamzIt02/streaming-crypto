

use std::fmt;
use std::ops::Deref;
use std::future::Future;
use std::pin::Pin;

use crate::{
    benchmarks::{
        bench_persists::save_json, bench_results::{BenchmarkResult, print_results}, bench_utils::dummy_master_key
    }, 
    stream::core::MasterKey
};

#[derive(Debug, PartialEq)]
pub enum ApiVersion {
    V2,
    V3,
}

impl ApiVersion {
    pub const fn as_str(&self) -> &'static str {
        match self {
            ApiVersion::V2 => "v2",
            ApiVersion::V3 => "v3",
        }
    }
}

// Deref makes the enum act like &str
impl Deref for ApiVersion {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl fmt::Display for ApiVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}


// Sync benchmarks
pub fn run_sync_benchmarks(_master_key: &MasterKey) -> Vec<BenchmarkResult> {
    let results = Vec::new();
    results
}

// Async benchmarks
pub async fn run_async_benchmarks(_master_key: &MasterKey) -> Vec<BenchmarkResult> {
    let results = Vec::new();
    results
}

// Type alias for async future
// pub type BenchFuture<'a> =
//     Pin<Box<dyn Future<Output = Vec<BenchmarkResult>> + Send + 'a>>;

pub type BenchFuture =
    Pin<Box<dyn Future<Output = Vec<BenchmarkResult>> + Send>>;

// Main orchestration
pub fn bench_main(
    api: ApiVersion,
    sync_benchmark_fn: Option<Box<dyn Fn() -> Vec<BenchmarkResult>>>,
    async_benchmark_fn: Option<Box<dyn Fn() -> BenchFuture + Send>>,
) {
    let master_key = dummy_master_key(); // own it

    let sync_fn = sync_benchmark_fn.unwrap_or_else(|| {
        let key = master_key.clone();
        Box::new(move || run_sync_benchmarks(&key))
    });

    let async_fn = async_benchmark_fn.unwrap_or_else(|| {
        let key = master_key.clone();
        Box::new(move || {
            let key = key.clone();
            Box::pin(async move {
                run_async_benchmarks(&key).await
            })
        })
    });

    println!("Running sync benchmarks...");
    let sync_results = sync_fn();
    print_results(&sync_results, "Sync benchmarks");

    let mut all_results = sync_results;

    println!("\nRunning async benchmarks...");
    let async_results = futures::executor::block_on(async_fn());
    print_results(&async_results, "Async benchmarks");
    
    all_results.extend(async_results);

    save_json(&all_results, None, Some(format!("results_{}", api).as_str()));
}

// ### 🔑 Key Notes
// - `BenchmarkResult` is a placeholder; adapt fields to our actual benchmark struct.
// - `run_sync_benchmarks` and `run_async_benchmarks` currently return empty vectors — we’ll plug in our real benchmark logic.
// - `bench_main` orchestrates sync and async runs, prints results, and saves JSON.
// - Uses `tokio::Runtime::block_on` to run async benchmarks inside a sync context, just like Python’s `asyncio.run`.
