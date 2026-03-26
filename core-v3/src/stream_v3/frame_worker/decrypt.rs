// # 📂 `src/stream_v3/frame_worker/decrypt.rs`

use std::time::Instant;

use bytes::{Bytes, BytesMut};
use crossbeam::channel::{Receiver, Sender};
use tracing::{error, info, warn};

use core_api::{
    crypto::{AadHeader, AeadImpl, build_aad, derive_nonce_12_tls_style},
    headers::HeaderV1, 
    stream_v2::{
        frame_worker::{DecryptedFrame, FrameWorkerError}, framing::{FrameHeader, FrameType, decode::{decode_frame, decode_in_place}}
    }, 
    telemetry::{Stage, StageTimes}, types::StreamError, utils::tracing_logger
};

use crate::stream_v3::pipeline::{Monitor, PipelineMonitor};

#[derive(Clone)]
pub struct DecryptFrameWorker3 {
    header: HeaderV1,
    aead: AeadImpl,
    monitor: Monitor,   // 👈 replaces fatal_tx + cancelled
}

impl DecryptFrameWorker3 {
    /// Creates a new frame decryption worker
    pub fn new(
        header: HeaderV1,
        session_key: &[u8],
        monitor: Monitor,
    ) -> Self {
        let aead = match AeadImpl::from_header_and_key(&header, session_key) {
            Ok(aead) => aead,
            Err(e) => {
                monitor.report_error(StreamError::FrameWorker(FrameWorkerError::Crypto(e)));
                panic!("[DECRYPT FRAME WORKER] AEAD init failed, pipeline cancelled");
            }
        };
        Self { header, aead, monitor }
    }

    // ### Zero‑Copy Decryption Implementation
    pub fn decrypt_in_place(&self, wire: &Bytes) -> Result<DecryptedFrame, FrameWorkerError> {
        let mut stage_times = StageTimes::default();

        // ---- Stage 1: Decode header ----
        let start = Instant::now();
        let view = decode_in_place(&wire)?;
        stage_times.add(Stage::Decode, start.elapsed());

        let ct_start = FrameHeader::LEN;
        let ct_end = ct_start + view.header.ciphertext_len() as usize;
        if ct_end > wire.len() {
            return Err(FrameWorkerError::InvalidInput(
                "Wire length mismatch: ciphertext extends beyond frame boundary".into(),
            ));
        }

        // ---- Stage 2: AAD + nonce ----
        let start = Instant::now();
        let aad_header = AadHeader {
            frame_type: view.header.frame_type().try_to_u8()?,
            segment_index: view.header.segment_index(),
            frame_index: view.header.frame_index(),
            payload_len: view.header.plaintext_len(),
        };
        let aad = build_aad(&self.header, &aad_header)?;
        let nonce = derive_nonce_12_tls_style(&self.header.salt, view.header.frame_index() as u64)?;
        stage_times.add(Stage::Validate, start.elapsed());

        // ---- Stage 3: Decrypt ----
        let start = Instant::now();
        // This buf is filled with encrypted [data, digest payload or terminator empty bytes]
        let mut buf = BytesMut::from(&wire[ct_start..ct_end]);
        match view.header.frame_type() {
            FrameType::Data => {
                self.aead.open_in_place(&nonce, &aad, &mut buf)?
            }
            FrameType::Digest => {}
            FrameType::Terminator => {}
        };
        stage_times.add(Stage::Decrypt, start.elapsed());

        Ok(DecryptedFrame {
            segment_index: view.header.segment_index(),
            frame_index: view.header.frame_index(),
            frame_type: view.header.frame_type(),
            wire: Bytes::from(""),             // Wire bytes moved (zero-copy)
            ct_range: ct_start..ct_end,
            plaintext: buf.freeze(),
            stage_times,
        })
    }

    /// Decrypts a single encrypted frame
    pub fn decrypt_frame(&self, wire: &Bytes) -> Result<DecryptedFrame, FrameWorkerError> {
        let mut stage_times = StageTimes::default();

        // ---- Stage 1: Decode header ----
        let start = Instant::now();
        let view = decode_frame(&wire)?;
        stage_times.add(Stage::Decode, start.elapsed());

        // ---- Stage 2: Validation + AAD ----
        let start = Instant::now();
        let ct_start = FrameHeader::LEN;
        let ct_end = ct_start + view.header.ciphertext_len() as usize;
        if ct_end > wire.len() {
            return Err(FrameWorkerError::InvalidInput(
                "Wire length mismatch: ciphertext extends beyond frame boundary".into(),
            ));
        }

        let aad_header = AadHeader {
            frame_type: view.header.frame_type().try_to_u8()?,
            segment_index: view.header.segment_index(),
            frame_index: view.header.frame_index(),
            payload_len: view.header.plaintext_len(),
        };
        let aad = build_aad(&self.header, &aad_header)?;
        let nonce = derive_nonce_12_tls_style(&self.header.salt, view.header.frame_index() as u64)?;
        stage_times.add(Stage::Validate, start.elapsed());

        // ---- Stage 3: AEAD Decryption ----
        let start = Instant::now();
        let plaintext: Vec<u8> = match view.header.frame_type() {
            FrameType::Data => self.aead.open(&nonce, &aad, view.ciphertext)?,
            FrameType::Digest => view.ciphertext.to_vec(),
            FrameType::Terminator => Vec::new(),
        };
        stage_times.add(Stage::Decrypt, start.elapsed());

        Ok(DecryptedFrame {
            segment_index: view.header.segment_index(),
            frame_index: view.header.frame_index(),
            frame_type: view.header.frame_type(),
            wire: Bytes::from(""),             // Wire bytes moved (zero-copy)
            ct_range: ct_start..ct_end,
            plaintext: Bytes::from(plaintext),
            stage_times,
        })
    }

    /// Runs the worker loop, processing frames until channel closes
    pub fn run(self, rx: Receiver<Bytes>, tx: Sender<DecryptedFrame>) {
        tracing_logger(Some(tracing::Level::DEBUG));

        info!("[DECRYPT FRAME WORKER] init, starting");
        loop {
            crossbeam::select! {
                recv(rx) -> msg => {
                    match msg {
                        Ok(input) => {
                            if self.monitor.is_cancelled() {
                                warn!("[DECRYPT FRAME WORKER] cancelled, exiting");
                                break;
                            }

                            match self.decrypt_in_place(&input) {
                                Ok(frame) => {
                                    if let Err(e) = tx.send(frame) {
                                        error!("[DECRYPT FRAME WORKER] tx send failed: {}", e);
                                        self.monitor.report_error(StreamError::FrameWorker(FrameWorkerError::StateError(e.to_string()),));
                                        break;
                                    }
                                }
                                Err(e) => {
                                    error!("[DECRYPT FRAME WORKER] decryption error: {:?}", e);
                                    self.monitor.report_error(StreamError::FrameWorker(e));
                                    break;
                                }
                            }
                        }
                        Err(_) => break, // channel closed
                    }
                },
                recv(self.monitor.cancel_rx()) -> _ => {
                    break;
                },
                default(std::time::Duration::from_millis(10)) => {
                    // 🔥 THIS is the real cancellation path
                    if self.monitor.is_cancelled() {
                        warn!("[DECRYPT FRAME WORKER] cancelled (timeout path), exiting");
                        break;
                    }
                }
            }
        }

        warn!("[DECRYPT FRAME WORKER] thread exiting");
    }
}
