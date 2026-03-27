// # 📂 `src/stream_v3/frame_worker/encrypt.rs`

use std::time::Instant;
use bytes::{Bytes, BytesMut};
use crossbeam::channel::{Receiver, Sender};
use tracing::{error, info, warn};

use crate::{
    crypto::{AadHeader, AeadImpl, build_aad, derive_nonce_12_tls_style}, 
    headers::HeaderV1, 
    stream::{
        frame_worker::{EncryptedFrame, FrameInput, FrameWorkerError}, framing::{FrameHeader, FrameType, encode::{encode_frame,encode_in_place}}
    }, 
    telemetry::{Stage, StageTimes}, 
    types::StreamError, utils::tracing_logger
};

use crate::v3::stream::pipeline::{Monitor, PipelineMonitor};

#[derive(Clone)]
pub struct EncryptFrameWorker3 {
    header: HeaderV1,
    aead: AeadImpl,
    monitor: Monitor,   // 👈 replaces fatal_tx + cancelled
}

impl EncryptFrameWorker3 {
    /// Creates a new frame encryption worker
    pub fn new(
        header: HeaderV1,
        session_key: &[u8],
        monitor: Monitor,
    ) -> Self {
        let aead = match AeadImpl::from_header_and_key(&header, session_key) {
            Ok(aead) => aead,
            Err(e) => {
                // Initialization failure is fatal → report via monitor
                monitor.report_error(StreamError::FrameWorker(FrameWorkerError::Crypto(e)));
                // Short‑circuit: monitor will cancel pipeline
                panic!("[ENCRYPT FRAME WORKER] AEAD init failed, pipeline cancelled");
            }
        };
        Self { header, aead, monitor }
    }
    // ### Zero‑Copy Implementation
    pub fn encrypt_in_place(&self, input: &FrameInput) -> Result<EncryptedFrame, FrameWorkerError> {
        let mut stage_times = StageTimes::default();

        // ---- Stage 1: Validation and AAD construction ----
        let start = Instant::now();
        input.validate()?;

        let plaintext_len = input.payload.len() as u32;
        let aad_header = AadHeader {
            frame_type: input.frame_type.try_to_u8()?,
            segment_index: input.segment_index,
            frame_index: input.frame_index,
            payload_len: plaintext_len,
        };

        let aad = build_aad(&self.header, &aad_header)?;
        let nonce = derive_nonce_12_tls_style(&self.header.salt, input.frame_index as u64)?;
        stage_times.add(Stage::Validate, start.elapsed());

        // ---- Stage 2: AEAD Encryption ----
        let start = Instant::now();

        // This buf is filled with input [data, digest payload or terminator empty bytes]
        let mut buf = BytesMut::from(&input.payload[..]);
        match input.frame_type {
            FrameType::Data => {
                // Works because BytesMut implements Buffer
                self.aead.seal_in_place(&nonce, &aad, &mut buf)?;
            }
            FrameType::Digest => {}
            FrameType::Terminator => {}
        }

        stage_times.add(Stage::Encrypt, start.elapsed());

        // ---- Stage 3: Frame header construction ----
        // Stage 3: Frame header construction
        let frame_header = FrameHeader::new(
            input.segment_index,
            input.frame_index,
            input.frame_type,
            plaintext_len,
            buf.len() as u32, // ciphertext length
        );

        // ---- Stage 4: Serialization ----
        let start = Instant::now();
        let mut wire = BytesMut::with_capacity(FrameHeader::LEN + buf.len());
        encode_in_place(&frame_header, &buf, &mut wire)?;
        stage_times.add(Stage::Encode, start.elapsed());

        let ct_start = FrameHeader::LEN;
        let ct_end = wire.len();

        Ok(EncryptedFrame {
            segment_index: frame_header.segment_index(),
            frame_index: frame_header.frame_index(),
            frame_type: frame_header.frame_type(),
            wire: wire.freeze(),
            ct_range: ct_start..ct_end,
            stage_times,
        })
    }

    /// Encrypts a single frame of data
    pub fn encrypt_frame(&self, input: &FrameInput) -> Result<EncryptedFrame, FrameWorkerError> {
        let mut stage_times = StageTimes::default();

        // ---- Stage 1: Validation + AAD ----
        let start = Instant::now();
        input.validate()?;

        let plaintext_len = input.payload.len() as u32;
        let aad_header = AadHeader {
            frame_type: input.frame_type.try_to_u8()?,
            segment_index: input.segment_index,
            frame_index: input.frame_index,
            payload_len: plaintext_len,
        };

        let aad = build_aad(&self.header, &aad_header)?;
        let nonce = derive_nonce_12_tls_style(&self.header.salt, input.frame_index as u64)?;
        stage_times.add(Stage::Validate, start.elapsed());

        // ---- Stage 2: AEAD Encryption ----
        let start = Instant::now();
        let ciphertext: Vec<u8> = match input.frame_type {
            FrameType::Data => self.aead.seal(&nonce, &aad, &input.payload)?,
            FrameType::Digest => input.payload.to_vec(),
            FrameType::Terminator => Vec::new(),
        };
        stage_times.add(Stage::Encrypt, start.elapsed());

        // ---- Stage 3: Frame header ----
        let frame_header = FrameHeader::new(
            input.segment_index,
            input.frame_index,
            input.frame_type,
            plaintext_len,
            ciphertext.len() as u32,
        );

        // ---- Stage 4: Serialization ----
        let start = Instant::now();
        let ct_start = FrameHeader::LEN;
        let wire = encode_frame(&frame_header, &ciphertext)?;
        let ct_end = wire.len();
        stage_times.add(Stage::Encode, start.elapsed());

        Ok(EncryptedFrame {
            segment_index: frame_header.segment_index(),
            frame_index: frame_header.frame_index(),
            frame_type: frame_header.frame_type(),
            wire: Bytes::from(wire),
            ct_range: ct_start..ct_end,
            stage_times,
        })
    }

    /// Runs the worker loop, processing frames until channel closes
    pub fn run(self, rx: Receiver<FrameInput>, tx: Sender<EncryptedFrame>) {
        tracing_logger(Some(tracing::Level::DEBUG));
       
        info!("[ENCRYPT FRAME WORKER] init, starting");
        loop {
            crossbeam::select! {
                recv(rx) -> msg => {
                    match msg {
                        Ok(input) => {
                            if self.monitor.is_cancelled() {
                                warn!("[ENCRYPT FRAME WORKER] cancelled, exiting");
                                break;
                            }

                            match self.encrypt_in_place(&input) {
                                Ok(frame) => {
                                    if let Err(e) = tx.send(frame) {
                                        error!("[ENCRYPT FRAME WORKER] tx send failed: {}", e);
                                        self.monitor.report_error(StreamError::FrameWorker(FrameWorkerError::StateError(e.to_string()),));
                                        break;
                                    }
                                }
                                Err(e) => {
                                    error!("[ENCRYPT FRAME WORKER] encryption error: {:?}", e);
                                    self.monitor.report_error(StreamError::FrameWorker(e));
                                    break;
                                }
                            }
                        }
                        Err(_) => break, // channel closed
                    }
                }
                recv(self.monitor.cancel_rx()) -> _ => {
                    break;
                }
                default(std::time::Duration::from_millis(10)) => {
                    // 🔥 THIS is the real cancellation path
                    if self.monitor.is_cancelled() {
                        warn!("[ENCRYPT FRAME WORKER] cancelled (timeout path), exiting");
                        break;
                    }
                }
            }
        }

        warn!("[ENCRYPT FRAME WORKER] thread exiting");
    }

}
