// # 📂 `tests/test_worker.rs`

#[cfg(test)]
mod worker_tests {
    use std::sync::{Arc, Mutex};
    use bytes::Bytes;
    use crossbeam::channel;

    use core_api::{
        compression::{CodecLevel, codec_ids},
        parallelism::Scheduler,
        stream_v2::{compression_worker::{CodecInfo, GpuCompressionBackend}, segmenting::types::SegmentFlags},
        telemetry::TelemetryEvent,
    };
    use core_v3::stream_v3::{compression_worker::run_compress_worker, pipeline::Monitor, segment_worker::SegmentInput};

    #[test]
    fn gpu_worker_handles_large_segment() {
        let codec_info = CodecInfo {
            codec_id: codec_ids::AUTO,
            level: CodecLevel::Custom(0),
            dict: None,
            gpu: None,
        };

        let backend = GpuCompressionBackend::new(codec_info).expect("gpu backend init");
        let (tx_in, rx_in) = channel::unbounded::<SegmentInput>();
        let (tx_out, rx_out) = channel::unbounded::<SegmentInput>();

        let scheduler = Arc::new(Mutex::new(Scheduler::new(0, 1, 4 * 1024 * 1024)));

        // create monitor
        let (monitor, monitor_rx) = Monitor::new(vec![], vec![]);

        // spawn worker
        let sched_clone = scheduler.clone();
        let monitor_clone = monitor.clone();
        std::thread::spawn(move || {
            run_compress_worker(rx_in, tx_out, Box::new(backend), sched_clone, monitor_clone)
        });

        // send a large segment (>= 4 MB)
        let big_buf = vec![42u8; 32 * 1024 * 1024]; // 32 MB
        let seg = SegmentInput {
            index: 0,
            bytes: Bytes::from(big_buf),
            flags: SegmentFlags::empty(),
            header: Default::default(),
        };
        tx_in.send(seg).unwrap();

        // receive result
        let result = rx_out.recv().unwrap();
        assert!(!result.bytes.is_empty(), "GPU worker should compress large segment successfully");

        // expect telemetry event, not error
        match monitor_rx.recv().unwrap() {
            Ok(TelemetryEvent::StageSnapshot { .. }) => {
                // success path: telemetry snapshot was reported
            }
            Err(e) => panic!("unexpected error reported: {:?}", e),
            other => panic!("unexpected monitor event: {:?}", other),
        }
    }
    
}
