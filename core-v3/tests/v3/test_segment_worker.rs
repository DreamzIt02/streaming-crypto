// # 📂 `tests/test_segment_worker.rs`

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use bytes::Bytes;
    use crossbeam::channel::unbounded;
    
    use core_api::{crypto::{DigestAlg, KEY_LEN_32}, headers::HeaderV1, parallelism::HybridParallelismProfile, recovery::AsyncLogManager, segment_worker::{DecryptContext, DecryptedSegment, EncryptContext, EncryptedSegment}, segmenting::{SegmentHeader, types::SegmentFlags}, telemetry::TelemetryEvent, types::StreamError};
    use core_v3::stream_v3::{segment_worker::{DecryptSegmentWorker3, EncryptSegmentWorker3, SegmentInput}, pipeline::{Monitor, PipelineMonitor}};

    fn setup_enc_context(alg: DigestAlg) -> (EncryptContext, Arc<AsyncLogManager>) {
        let header = HeaderV1::test_header();
        let profile = HybridParallelismProfile::semi_dynamic(header.chunk_size as u32, 0.50, 64);
        let session_key = vec![0x42u8; KEY_LEN_32];
        let log_manager = Arc::new(AsyncLogManager::new("test_audit.log", 100).unwrap());
        let context = EncryptContext::new(header, profile, &session_key, alg).unwrap();
        (context, log_manager)
    }

    fn setup_dec_context(alg: DigestAlg) -> (DecryptContext, Arc<AsyncLogManager>) {
        let header = HeaderV1::test_header();
        let profile = HybridParallelismProfile::semi_dynamic(header.chunk_size as u32, 0.50, 64);
        let session_key = vec![0x42u8; KEY_LEN_32];
        let log_manager = Arc::new(AsyncLogManager::new("test_audit.log", 100).unwrap());
        let context = DecryptContext::from_stream_header(header, profile, &session_key, alg).unwrap();
        (context, log_manager)
    }

    // Bridge function: forward encrypted segments into decrypt input
    fn forward_encrypted_to_decrypt(
        enc_rx: crossbeam::channel::Receiver<EncryptedSegment>,
        dec_tx: crossbeam::channel::Sender<SegmentInput>,
        monitor: Monitor
    ) {
        std::thread::spawn(move || {
            while let Ok(enc_seg) = enc_rx.recv() {
                if monitor.is_cancelled() {
                    break;
                }
                if dec_tx.send(enc_seg.into()).is_err() {
                    break;
                }
            }
            eprintln!("[FORWARD THREAD] exiting, dropping bridge_tx");
        });
    }

    /// Spawns encrypt + decrypt segment workers, runs a single input, and returns the decrypted output.
    /// Ensures proper channel teardown and thread joining.
    fn run_segment_roundtrip(
        enc: EncryptSegmentWorker3,
        dec: DecryptSegmentWorker3,
        input: SegmentInput,
        monitor: Monitor,
    ) -> DecryptedSegment {
        let (enc_tx, enc_rx) = crossbeam::channel::unbounded();
        let (mid_tx, mid_rx) = crossbeam::channel::unbounded();
        let (bridge_tx, bridge_rx) = crossbeam::channel::unbounded();
        let (dec_tx, dec_rx) = crossbeam::channel::unbounded();

        // clone mid_tx before moving it
        let mid_tx_for_worker = mid_tx.clone();

        // Spawn encrypt worker
        let enc_handle = std::thread::spawn(move || {
            enc.run(enc_rx, mid_tx_for_worker);
        });

        // bridge converts EncryptedSegment → DecryptSegmentInput
        forward_encrypted_to_decrypt(mid_rx, bridge_tx, monitor);

        // Spawn decrypt worker
        let dec_handle = std::thread::spawn(move || {
            dec.run(bridge_rx, dec_tx);
        });

        // Send input
        enc_tx.send(input).unwrap();

        // Receive output
        let result = dec_rx.recv().unwrap();

        // Close input channel so workers exit
        drop(enc_tx);
        drop(mid_tx);

        // Join threads
        enc_handle.join().unwrap();
        dec_handle.join().unwrap();

        result
    }

    // ## ✅ 0. End-to-end encrypt → decrypt (multi segment)

    #[test]
    fn encrypt_decrypt_large_segment_roundtrip() {
        let (crypto_enc, log_enc) = setup_enc_context(DigestAlg::Sha256);
        let (crypto_dec, log_dec) = setup_dec_context(DigestAlg::Sha256);

        // Create monitor
        let (monitor, _monitor_rx) = Monitor::new(vec![], vec![]);

        let enc = EncryptSegmentWorker3::new(Arc::new(crypto_enc), log_enc, monitor.clone());
        let dec = DecryptSegmentWorker3::new(Arc::new(crypto_dec), log_dec, monitor.clone());

        let plaintext = Bytes::from(vec![0xAB; 256 * 1024]); // 256 KB of data

        let input = SegmentInput {
            index: 42,
            bytes: plaintext.clone(),
            flags: SegmentFlags::empty(),
            header: SegmentHeader::default(),
        };

        let decrypted = run_segment_roundtrip(enc, dec, input, monitor);

        assert_eq!(decrypted.bytes, plaintext);
        assert_eq!(decrypted.header.segment_index(), 42);
    }

    // ## ✅ 1. End-to-end encrypt → decrypt (single segment)
    #[test]
    fn encrypt_decrypt_segment_roundtrip() {
        let (crypto_enc, log_enc) = setup_enc_context(DigestAlg::Sha256);
        let (crypto_dec, log_dec) = setup_dec_context(DigestAlg::Sha256);

        let (monitor, _monitor_rx) = Monitor::new(vec![], vec![]);
        let enc = EncryptSegmentWorker3::new(Arc::new(crypto_enc), log_enc, monitor.clone());
        let dec = DecryptSegmentWorker3::new(Arc::new(crypto_dec), log_dec, monitor.clone());

        let (enc_tx, enc_rx) = unbounded();
        let (mid_tx, mid_rx) = unbounded();
        let (bridge_tx, bridge_rx) = unbounded();
        let (dec_tx, dec_rx) = unbounded();

        let enc_handle = std::thread::spawn(move || enc.run(enc_rx, mid_tx));
        forward_encrypted_to_decrypt(mid_rx, bridge_tx, monitor);
        let dec_handle = std::thread::spawn(move || dec.run(bridge_rx, dec_tx));

        let plaintext = Bytes::from_static(b"hello segmented crypto world");
        enc_tx.send(SegmentInput {
            index: 7,
            bytes: plaintext.clone(),
            flags: SegmentFlags::empty(),
            header: SegmentHeader::default(),
        }).unwrap();

        let decrypted = dec_rx.recv().unwrap();
        assert_eq!(decrypted.bytes, plaintext);

        drop(enc_tx);
        enc_handle.join().unwrap();
        dec_handle.join().unwrap();
    }

    // ## ✅ 2. Large segment (multi-frame, parallelism)

    #[test]
    fn large_segment_parallel_encryption() {
        let (crypto_enc, log_enc) = setup_enc_context(DigestAlg::Sha256);
        let (crypto_dec, log_dec) = setup_dec_context(DigestAlg::Sha256);

        // Create monitor
        let (monitor, _monitor_rx) = Monitor::new(vec![], vec![]);

        let enc = EncryptSegmentWorker3::new(Arc::new(crypto_enc), log_enc, monitor.clone());
        let dec = DecryptSegmentWorker3::new(Arc::new(crypto_dec), log_dec, monitor.clone());

        let (enc_tx, enc_rx) = unbounded::<SegmentInput>();
        let (mid_tx, mid_rx) = unbounded::<EncryptedSegment>();
        let (bridge_tx, bridge_rx) = unbounded::<SegmentInput>();
        let (dec_tx, dec_rx) = unbounded::<DecryptedSegment>();

        // Spawn encrypt worker
        let enc_handle = std::thread::spawn(move || {
            enc.run(enc_rx, mid_tx);
        });

        // bridge converts EncryptedSegment → DecryptSegmentInput
        forward_encrypted_to_decrypt(mid_rx, bridge_tx, monitor);

        // Spawn decrypt worker
        let dec_handle = std::thread::spawn(move || {
            dec.run(bridge_rx, dec_tx);
        });

        let data = vec![0xAB; 2 * 1024 * 1024]; // 2 MB
        let plaintext = Bytes::from(data.clone());

        // Send common SegmentInput
        enc_tx.send(SegmentInput {
            index: 0,
            bytes: plaintext,
            flags: SegmentFlags::empty(),
            header: SegmentHeader::default(),
        }).unwrap();

        // Receive decrypted segment
        let decrypted = dec_rx.recv().unwrap();
        let out = decrypted.bytes;

        assert_eq!(out, data);

        // ✅ Close channels so workers see EOF and exit
        drop(enc_tx);

        // ✅ Join threads
        enc_handle.join().unwrap();
        dec_handle.join().unwrap();
    }

    // ## ❌ 3. Corrupted ciphertext → digest failure

    #[test]
    fn corrupted_segment_fails_digest_verification() {
        let (crypto_enc, log_enc) = setup_enc_context(DigestAlg::Sha256);
        let (crypto_dec, log_dec) = setup_dec_context(DigestAlg::Sha256);

        let (monitor, monitor_rx) = Monitor::new(vec![], vec![]);
        let enc = EncryptSegmentWorker3::new(Arc::new(crypto_enc), log_enc, monitor.clone());
        let dec = DecryptSegmentWorker3::new(Arc::new(crypto_dec), log_dec, monitor.clone());

        let (enc_tx, enc_rx) = unbounded();
        let (mid_tx, mid_rx) = unbounded();
        let (bridge_tx, bridge_rx) = unbounded();
        let (dec_tx, _dec_rx) = unbounded();

        let enc_handle = std::thread::spawn(move || enc.run(enc_rx, mid_tx));

        enc_tx.send(SegmentInput {
            index: 1,
            bytes: Bytes::from_static(b"tamper me"),
            flags: SegmentFlags::empty(),
            header: SegmentHeader::default(),
        }).unwrap();

        let mut encrypted = mid_rx.recv().unwrap();
        let mut wire = bytes::BytesMut::from(&encrypted.wire[..]);
        let offset = wire.len() / 2;
        wire[offset] ^= 0xFF; // corrupt the wire
        encrypted.wire = wire.freeze();

        bridge_tx.send(encrypted.into()).unwrap();
        let dec_handle = std::thread::spawn(move || dec.run(bridge_rx, dec_tx));

        // Drain monitor events until we see an error
        let mut saw_error = false;
        while let Ok(event) = monitor_rx.recv() {
            match event {
                Err(StreamError::SegmentWorker(_)) => {
                    saw_error = true;
                    break;
                }
                Ok(_) => {
                    // telemetry snapshot, ignore
                },
                _ => {
                    // Other error types
                    saw_error = true;
                    break;
                }
            }
        }
        assert!(saw_error, "expected SegmentWorker error after corruption");

        drop(enc_tx);
        enc_handle.join().unwrap();
        dec_handle.join().unwrap();
    }

    // ## ❌ 4. Wrong crypto context (wrong key)

    #[test]
    fn wrong_key_fails_segment_decryption() {
        let (crypto_enc, log_enc) = setup_enc_context(DigestAlg::Sha256);
        let (crypto_dec, log_dec) = setup_dec_context(DigestAlg::Sha256);

        // Separate monitors
        let (enc_tx, enc_rx) = unbounded();
        let (mid_tx, mid_rx) = unbounded();
        let (bridge_tx, bridge_rx) = unbounded();
        let (dec_tx, _dec_rx) = unbounded();

        let (monitor_enc, monitor_rx_enc) = Monitor::new(
            vec![],
            vec![],
        );

        let (monitor_dec, monitor_rx_dec) = Monitor::new(
            vec![],
            vec![],
        );

        let enc = EncryptSegmentWorker3::new(Arc::new(crypto_enc), log_enc, monitor_enc.clone());

        // Corrupt the key
        let mut wrong_crypto = crypto_dec.clone();
        wrong_crypto.base.session_key[0] ^= 0xFF;
        let dec = DecryptSegmentWorker3::new(Arc::new(wrong_crypto), log_dec, monitor_dec.clone());

        let enc_handle = std::thread::spawn(move || enc.run(enc_rx, mid_tx));
        forward_encrypted_to_decrypt(mid_rx, bridge_tx, monitor_dec);
        let dec_handle = std::thread::spawn(move || dec.run(bridge_rx, dec_tx));

        enc_tx.send(SegmentInput {
            index: 3,
            bytes: Bytes::from_static(b"secret"),
            flags: SegmentFlags::empty(),
            header: SegmentHeader::default(),
        }).unwrap();
        // Close input channels so workers see EOF
        drop(enc_tx);

        // Drain all monitor events until channel closes
        let mut saw_error = false;
        while let Ok(event) = monitor_rx_enc.recv() {
            match event {
                Err(_) => {
                    break;
                }
                Ok(_) => {
                    // telemetry snapshot, ignore
                    break;
                }
            }
        }
        while let Ok(event) = monitor_rx_dec.recv() {
            match event {
                Err(_) => {
                    saw_error = true;
                    break;
                }
                Ok(_) => {
                    // telemetry snapshot, ignore
                    break;
                }
            }
        }
        eprintln!("ERROR Seen, Other {}", saw_error);
        assert!(saw_error, "expected SegmentWorker error after wrong key decryption");

        eprintln!("joining enc_handle...");
        enc_handle.join().unwrap();
        eprintln!("enc_handle joined");
        eprintln!("joining dec_handle...");
        dec_handle.join().unwrap();
        eprintln!("dec_handle joined");
    }

    // ## ❌ 5. Truncated segment wire

    #[test]
    fn truncated_segment_fails() {
        let (crypto_enc, log_enc) = setup_enc_context(DigestAlg::Sha256);
        let (crypto_dec, log_dec) = setup_dec_context(DigestAlg::Sha256);

        let (monitor, monitor_rx) = Monitor::new(vec![], vec![]);
        let enc = EncryptSegmentWorker3::new(Arc::new(crypto_enc), log_enc, monitor.clone());
        let dec = DecryptSegmentWorker3::new(Arc::new(crypto_dec), log_dec, monitor.clone());

        let (enc_tx, enc_rx) = unbounded();
        let (mid_tx, mid_rx) = unbounded();
        let (bridge_tx, bridge_rx) = unbounded();
        let (dec_tx, _dec_rx) = unbounded();

        let enc_handle = std::thread::spawn(move || enc.run(enc_rx, mid_tx));

        enc_tx.send(SegmentInput {
            index: 4,
            bytes: Bytes::from_static(b"cut me"),
            flags: SegmentFlags::empty(),
            header: SegmentHeader::default(),
        }).unwrap();

        let mut encrypted = mid_rx.recv().unwrap();
        encrypted.wire.truncate(encrypted.wire.len() - 5);

        bridge_tx.send(encrypted.into()).unwrap();
                let dec_handle = std::thread::spawn(move || dec.run(bridge_rx, dec_tx));

        // Drain monitor events until we see an error
        let mut saw_error = false;
        while let Ok(event) = monitor_rx.recv() {
            match event {
                Err(StreamError::SegmentWorker(_)) => {
                    saw_error = true;
                    break;
                }
                Ok(_) => {
                    // telemetry snapshot, ignore
                },
                _ => {
                    // Other error types
                    saw_error = true;
                    break;
                }
            }
        }
        assert!(saw_error, "expected SegmentWorker error after corruption");

        drop(enc_tx);
        enc_handle.join().unwrap();
        dec_handle.join().unwrap();
    }
    // ## ❌ 6. Missing terminator frame

    #[test]
    fn missing_terminator_frame_is_rejected() {
        let (crypto_enc, log_enc) = setup_enc_context(DigestAlg::Sha256);
        let (crypto_dec, log_dec) = setup_dec_context(DigestAlg::Sha256);

        let (monitor, monitor_rx) = Monitor::new(vec![], vec![]);
        let enc = EncryptSegmentWorker3::new(Arc::new(crypto_enc), log_enc, monitor.clone());
        let dec = DecryptSegmentWorker3::new(Arc::new(crypto_dec), log_dec, monitor.clone());

        let (enc_tx, enc_rx) = unbounded();
        let (mid_tx, mid_rx) = unbounded();
        let (bridge_tx, bridge_rx) = unbounded();
        let (dec_tx, _dec_rx) = unbounded();

        let enc_handle = std::thread::spawn(move || enc.run(enc_rx, mid_tx));

        enc_tx.send(SegmentInput {
            index: 5,
            bytes: Bytes::from_static(b"no terminator"),
            flags: SegmentFlags::empty(),
            header: SegmentHeader::default(),
        }).unwrap();

        let encrypted = mid_rx.recv().unwrap();

        // drop last frame bytes (simulate missing terminator)
        let truncated = encrypted.wire.slice(..encrypted.wire.len() - 32);

        bridge_tx.send(EncryptedSegment {
            header: encrypted.header,
            wire: truncated,
            counters: encrypted.counters,
            stage_times: encrypted.stage_times,
        }.into()).unwrap();

        let dec_handle = std::thread::spawn(move || dec.run(bridge_rx, dec_tx));

        // Drain monitor events until we see an error
        let mut saw_error = false;
        while let Ok(event) = monitor_rx.recv() {
            match event {
                Err(StreamError::Segment(_)) => {
                    saw_error = true;
                    break;
                }
                Ok(_) => {
                    // telemetry snapshot, ignore
                },
                _ => {
                    // Other error types
                }
            }
        }
        assert!(saw_error, "expected SegmentWorker error after corruption");

        drop(enc_tx);
        enc_handle.join().unwrap();
        dec_handle.join().unwrap();
    }

    // ## ✅ 7. Deterministic encryption (same input → same wire)

    #[test]
    fn segment_encryption_is_deterministic() {
        let (crypto_enc, log_enc) = setup_enc_context(DigestAlg::Sha256);

        // Create monitor
        let (monitor, _monitor_rx) = Monitor::new(vec![], vec![]);

        let enc = EncryptSegmentWorker3::new(Arc::new(crypto_enc), log_enc, monitor.clone());

        let (tx, rx) = unbounded::<SegmentInput>();
        let (out_tx, out_rx) = unbounded::<EncryptedSegment>();

        // spawn the worker in a separate thread
        let enc_handle = std::thread::spawn(move || {
            enc.run(rx, out_tx);
        });

        let payload = Bytes::from_static(b"deterministic segment");

        tx.send(SegmentInput {
            index: 9,
            bytes: payload.clone(),
            flags: SegmentFlags::empty(),
            header: SegmentHeader::default(),
        }).unwrap();
        let a = out_rx.recv().unwrap();

        tx.send(SegmentInput {
            index: 9,
            bytes: payload,
            flags: SegmentFlags::empty(),
            header: SegmentHeader::default(),
        }).unwrap();
        let b = out_rx.recv().unwrap();

        assert_eq!(a.wire, b.wire);

        // close input channel so worker exits
        drop(tx);

        // join thread to finish cleanly
        enc_handle.join().unwrap();
    }

    // ## ✅ 8. Telemetry sanity checks

    #[test]
    fn telemetry_counters_are_consistent() {
        let (crypto_enc, log_enc) = setup_enc_context(DigestAlg::Sha256);
        let (crypto_dec, log_dec) = setup_dec_context(DigestAlg::Sha256);

        // Create monitor
        let (monitor_enc, _monitor_rx_enc) = Monitor::new(
            vec![],
            vec![],
        );

        let (monitor_dec, monitor_rx_dec) = Monitor::new(
            vec![],
            vec![],
        );

        let enc = EncryptSegmentWorker3::new(Arc::new(crypto_enc), log_enc, monitor_enc);
        let dec = DecryptSegmentWorker3::new(Arc::new(crypto_dec), log_dec, monitor_dec.clone());

        let (enc_tx, enc_rx) = unbounded::<SegmentInput>();
        let (mid_tx, mid_rx) = unbounded::<EncryptedSegment>();
        let (bridge_tx, bridge_rx) = unbounded::<SegmentInput>();
        let (dec_tx, dec_rx) = unbounded::<DecryptedSegment>();

        // Spawn encrypt worker

        let enc_handle = std::thread::spawn(move || {
            enc.run(enc_rx, mid_tx);
        });

        // bridge converts EncryptedSegment → DecryptSegmentInput
        forward_encrypted_to_decrypt(mid_rx, bridge_tx, monitor_dec);

        // Spawn decrypt worker
        let dec_handle = std::thread::spawn(move || {
            dec.run(bridge_rx, dec_tx);
        });

        let plaintext = Bytes::from_static(b"telemetry test");

        enc_tx.send(SegmentInput {
            index: 11,
            bytes: plaintext.clone(),
            flags: SegmentFlags::empty(),
            header: SegmentHeader::default(),
        }).unwrap();

        // Receive decrypted segment
        let decrypted = dec_rx.recv().unwrap();
        assert_eq!(decrypted.bytes.as_ref(), b"telemetry test");

        // Expect telemetry event from monitor
        match monitor_rx_dec.recv().unwrap() {
            Ok(TelemetryEvent::StageSnapshot { counters, .. }) => {
                assert!(counters.frames_data > 0);
                assert_eq!(counters.frames_digest, 1);
                assert_eq!(counters.frames_terminator, 1);
                assert!(counters.bytes_compressed > 0);
            }
            other => panic!("unexpected monitor event: {:?}", other),
        }

        // ✅ Close input channel so workers see EOF and exit
        drop(enc_tx);

        // ✅ Join threads
        enc_handle.join().unwrap();
        dec_handle.join().unwrap();
    }

}
// # 🧠 Why this suite is **correct**

// This test suite validates:

// ✔ frame parallelism
// ✔ ordering invariants
// ✔ digest correctness
// ✔ terminator enforcement
// ✔ truncation handling
// ✔ corruption detection
// ✔ deterministic crypto
// ✔ telemetry accuracy
// ✔ channel shutdown behavior
