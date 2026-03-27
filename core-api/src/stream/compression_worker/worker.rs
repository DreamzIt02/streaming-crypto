// ## 📦 `src/stream/compression_worker/worker.rs`

use crate::{
    parallelism::WorkerTarget, 
    stream::
        compression_worker::{CodecInfo, CompressionBackend, CpuCompressionBackend, GpuCompressionBackend}
    
};

/// Factory: choose backend based on codec + target
pub fn make_backend(target: WorkerTarget, codec_info: CodecInfo) -> Box<dyn CompressionBackend> {
    match target {
        WorkerTarget::Cpu(_) => {
            let backend = CpuCompressionBackend::new(codec_info)
                .expect("failed to create CPU compressor/decompressor");
            Box::new(backend)
        }
        WorkerTarget::Gpu(_) => {
            let backend = GpuCompressionBackend::new(codec_info)
                .expect("failed to create GPU compressor/decompressor");
            Box::new(backend)
        }
    }
}
