pub mod compression;
pub mod pipeline;
pub mod types;

pub use compression:: {
    spawn_compression_workers, spawn_decompression_workers,
    spawn_compression_workers_scoped, spawn_decompression_workers_scoped,
};

pub use types:: {
    PipelineConfig, PipelineCancellation
};

pub use pipeline:: {
    encrypt_pipeline, decrypt_pipeline
};