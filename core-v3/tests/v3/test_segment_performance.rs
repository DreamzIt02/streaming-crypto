// # 📂 `tests/test_segment_performance.rs`

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use bytes::Bytes;
    use crossbeam::channel::unbounded;

    use core_api::{crypto::{DigestAlg, KEY_LEN_32},
        headers::HeaderV1, parallelism::HybridParallelismProfile, recovery::AsyncLogManager, segmenting::SegmentHeader, 
        stream::{segment_worker::{DecryptContext, DecryptedSegment, EncryptContext, EncryptedSegment, SegmentWorkerError}, segmenting::types::SegmentFlags}, types::StreamError};
    use core_v3::stream_v3::{pipeline::{Monitor, PipelineMonitor}, segment_worker::{DecryptSegmentWorker3, EncryptSegmentWorker3, SegmentInput}};

    fn setup_enc_context(alg: DigestAlg, chunk_size: usize) -> (EncryptContext, Arc<AsyncLogManager>) {
        let header = HeaderV1{ chunk_size: chunk_size as u32, ..HeaderV1::test_header() }; // Mock header
        let profile = HybridParallelismProfile::semi_dynamic(header.chunk_size as u32, 0.50, 64);
       // Create a Vec of 32 bytes
        let session_key = vec![0x42u8; KEY_LEN_32];
        let log_manager = Arc::new(AsyncLogManager::new("test_audit.log", 100).unwrap());
        
        let context = EncryptContext::new(
            header,
            profile,
            &session_key,
            alg,
        ).unwrap();
        (context, log_manager)
    }

    fn setup_dec_context(alg: DigestAlg, chunk_size: usize) -> (DecryptContext, Arc<AsyncLogManager>) {
        let header = HeaderV1{ chunk_size: chunk_size as u32, ..HeaderV1::test_header() }; // Mock header
        let profile = HybridParallelismProfile::semi_dynamic(header.chunk_size as u32, 0.50, 64);
       // Create a Vec of 32 bytes
        let session_key = vec![0x42u8; KEY_LEN_32];
        let log_manager = Arc::new(AsyncLogManager::new("test_audit.log", 100).unwrap());
        
        let context = DecryptContext::from_stream_header(
            header,
            profile,
            &session_key,
            alg,
        ).unwrap();
        (context, log_manager)
    }

    fn run_segment_encrypt(
        enc: EncryptSegmentWorker3,
        input: SegmentInput,
    ) -> Result<EncryptedSegment, StreamError> {
        let (enc_tx, enc_rx) = crossbeam::channel::unbounded();
        let (mid_tx, mid_rx) = crossbeam::channel::unbounded();

        let enc_handle = std::thread::spawn(move || {
            enc.run(enc_rx, mid_tx);
        });

        enc_tx.send(input).map_err(|_| StreamError::ChannelSend)?;
        drop(enc_tx);

        let enc_result = mid_rx.recv().unwrap();

        drop(mid_rx);

        enc_handle.join().map_err(|_| StreamError::ThreadPanic)?;

        Ok(enc_result)
    }

    fn run_segment_decrypt(
        dec: DecryptSegmentWorker3,
        enc_seg: EncryptedSegment,
    ) -> Result<DecryptedSegment, StreamError> {
        let (bridge_tx, bridge_rx) = crossbeam::channel::unbounded();
        let (dec_tx, dec_rx) = crossbeam::channel::unbounded();

        let dec_handle = std::thread::spawn(move || {
            dec.run(bridge_rx, dec_tx);
        });

        let dec_in: SegmentInput = enc_seg.into();
        bridge_tx.send(dec_in).map_err(|_| StreamError::ChannelSend)?;
        drop(bridge_tx);

        let dec_result = dec_rx.recv().unwrap();

        drop(dec_rx);

        dec_handle.join().map_err(|_| StreamError::ThreadPanic)?;

        Ok(dec_result)
    }
    
    // Bridge function: forward encrypted segments into decrypt input
    fn forward_encrypted_to_decrypt(
        enc_rx: crossbeam::channel::Receiver<EncryptedSegment>,
        dec_tx: crossbeam::channel::Sender<SegmentInput>,
        monitor: Monitor,
    ) -> crossbeam::channel::Receiver<EncryptedSegment> {
        // Channel to return encrypted segments back to caller (for inspection in tests)
        let (result_tx, result_rx) = unbounded();

        std::thread::spawn(move || {
            while let Ok(enc_seg) = enc_rx.recv() {
                // forward to decrypt
                let dec_in: SegmentInput = enc_seg.clone().into();
                if dec_tx.send(dec_in).is_err() {
                    // downstream closed
                    monitor.report_error(StreamError::SegmentWorker(
                        SegmentWorkerError::StateError("decrypt input closed".into()),
                    ));
                    break;
                }
                // send encrypted segment back to caller
                if result_tx.send(enc_seg).is_err() {
                    // caller dropped
                    break;
                }
            }
            // when enc_rx closes, worker exits
        });

        result_rx
    }

    /// Spawns encrypt + decrypt segment workers, runs a single input, and returns the decrypted output.
    /// Ensures proper channel teardown and thread joining.
    fn run_segment_roundtrip(
        enc: EncryptSegmentWorker3,
        dec: DecryptSegmentWorker3,
        input: SegmentInput,
        _monitor_enc: Monitor,
        monitor_dec: Monitor,
    ) -> Result<(EncryptedSegment, DecryptedSegment), StreamError> {
        let (enc_tx, enc_rx) = crossbeam::channel::unbounded();
        let (mid_tx, mid_rx) = crossbeam::channel::unbounded();
        let (bridge_tx, bridge_rx) = crossbeam::channel::unbounded();
        let (dec_tx, dec_rx) = crossbeam::channel::unbounded();

        // Spawn encrypt worker
        let enc_handle = std::thread::spawn(move || {
            enc.run(enc_rx, mid_tx);
        });

        // Spawn decrypt worker
        let dec_handle = std::thread::spawn(move || {
            dec.run(bridge_rx, dec_tx);
        });

        // Spawn bridge
        let result_rx = forward_encrypted_to_decrypt(mid_rx, bridge_tx, monitor_dec);

        // ✅ Send input FIRST
        enc_tx.send(input).unwrap();

        // Close input so encrypt worker exits after 1 segment
        drop(enc_tx);

        // Receive encrypted segment
        let enc_result = result_rx.recv().unwrap();

        // Receive decrypted segment
        let dec_result = dec_rx
            .recv()
            .unwrap();

        // Close receivers so bridge exits
        drop(result_rx);
        drop(dec_rx);

        enc_handle.join().unwrap();
        dec_handle.join().unwrap();

        Ok((enc_result, dec_result))
    }
    
    #[test]
    fn encrypt_large_segment() {
        let chunk_size = 4 * 1024 * 1024; // 4 MB
        let (crypto_enc, log_enc) = setup_enc_context(DigestAlg::Blake3, chunk_size);
        let (crypto_dec, log_dec) = setup_dec_context(DigestAlg::Blake3, chunk_size);

        let (monitor_enc, _monitor_rx_enc) = Monitor::new(
            vec![],
            vec![],
        );

        let (monitor_dec, _monitor_rx_dec) = Monitor::new(
            vec![],
            vec![],
        );

        let enc = EncryptSegmentWorker3::new(
            Arc::new(crypto_enc),
            log_enc,
            monitor_enc,
        );
        let dec = DecryptSegmentWorker3::new(
            Arc::new(crypto_dec),
            log_dec,
            monitor_dec,
        );

         // Step 1: initialize workers once
        let plaintext = Bytes::from(vec![0xAB; chunk_size]);

        let input = SegmentInput {
            index: 42,
            bytes: plaintext.clone(),
            flags: SegmentFlags::empty(),
            header: SegmentHeader::default(),
        };

        let encrypted = run_segment_encrypt(enc, input).unwrap();

        assert!(encrypted.wire.len() > plaintext.len());

        let decrypted = run_segment_decrypt(dec, encrypted.clone()).unwrap();

        assert_eq!(decrypted.bytes, plaintext);

        println!("{}", encrypted.stage_times);
        println!("{}", decrypted.stage_times);

    }

    #[test]
    fn encrypt_decrypt_large_segment_roundtrip() {
        let chunk_size = 4 * 1024 * 1024; // 4 MB
        let (crypto_enc, log_enc) = setup_enc_context(DigestAlg::Blake3, chunk_size);
        let (crypto_dec, log_dec) = setup_dec_context(DigestAlg::Blake3, chunk_size);

        let (monitor_enc, _monitor_rx_enc) = Monitor::new(
            vec![],
            vec![],
        );

        let (monitor_dec, _monitor_rx_dec) = Monitor::new(
            vec![],
            vec![],
        );

        let enc = EncryptSegmentWorker3::new(
            Arc::new(crypto_enc),
            log_enc,
            monitor_enc.clone(),
        );
        let dec = DecryptSegmentWorker3::new(
            Arc::new(crypto_dec),
            log_dec,
            monitor_dec.clone(),
        );

        let plaintext = Bytes::from(vec![0xAB; chunk_size]);

        let input = SegmentInput {
            index: 42,
            bytes: plaintext.clone(),
            flags: SegmentFlags::empty(),
            header: SegmentHeader::default(),
        };

        let (encrypted, decrypted) = run_segment_roundtrip(enc, dec, input, monitor_enc, monitor_dec).unwrap();

        assert_eq!(decrypted.bytes, plaintext);
        assert_eq!(decrypted.header.segment_index(), 42);

        println!("{}", encrypted.counters);
        println!("{}", encrypted.stage_times);
        println!("{}", decrypted.counters);
        println!("{}", decrypted.stage_times);

    }

}
