// # 📂 `src/stream_v2/frame_worker/encrypt.rs`

use std::{sync::{Arc, atomic::{AtomicBool, Ordering}}, time::Instant};
use bytes::{Bytes, BytesMut};
use crossbeam::channel::{Receiver, Sender};
use tracing::{error, info, warn};

use crate::{
    crypto::{AadHeader, AeadImpl, build_aad, derive_nonce_12_tls_style}, headers::HeaderV1, 
    stream::{
        frame_worker::{EncryptedFrame, FrameInput, FrameWorkerError}, 
        framing::{FrameHeader, FrameType, encode::{encode_frame, encode_in_place}}
    }, telemetry::{Stage, StageTimes}, types::StreamError, utils::tracing_logger
};

pub struct EncryptFrameWorker1 {
    header: HeaderV1,
    aead: AeadImpl,
    fatal_tx: Sender<StreamError>,        // global error channel
    cancelled: Arc<AtomicBool>,           // global cancellation flag
}

impl EncryptFrameWorker1 {
    /// Creates a new frame encryption worker
    ///
    /// # Arguments
    /// * `header` - Stream header containing encryption parameters
    /// * `session_key` - Session key for AEAD encryption
    /// * `fatal_tx` - Channel to signal fatal errors to the pipeline monitor
    /// * `cancelled` - Shared cancellation flag for graceful shutdown
    pub fn new(
        header: HeaderV1,
        session_key: &[u8],
        fatal_tx: Sender<StreamError>,
        cancelled: Arc<AtomicBool>,
    ) -> Result<Self, FrameWorkerError> {
        let aead = AeadImpl::from_header_and_key(&header, session_key)?;
        Ok(Self {
            header,
            aead,
            fatal_tx,
            cancelled,
        })
    }

    /// Encrypts a single frame of data
    ///
    /// # Process
    /// 1. Validates input and builds AAD (Additional Authenticated Data)
    /// 2. Derives frame-specific nonce
    /// 3. Performs AEAD encryption (or skips for Terminator frames)
    /// 4. Constructs frame header with ciphertext metadata
    /// 5. Serializes frame header + ciphertext into wire format
    pub fn encrypt_frame(&self, input: &FrameInput) -> Result<EncryptedFrame, FrameWorkerError> {
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

        // Build AAD from immutable fields
        let aad = build_aad(&self.header, &aad_header)?;

        // Derive nonce using frame index to ensure uniqueness
        let nonce = derive_nonce_12_tls_style(&self.header.salt, input.frame_index as u64)?;
        stage_times.add(Stage::Validate, start.elapsed());

        // ---- Stage 2: AEAD Encryption ----
        let start = Instant::now();
        let ciphertext: Vec<u8> = match input.frame_type {
            FrameType::Data => {
                // Normal encryption path for data
                self.aead.seal(&nonce, &aad, &input.payload)?
            }
            FrameType::Digest => {
                // Digest payload is already a hash, no AEAD needed
                input.payload.to_vec()
            }
            FrameType::Terminator => {
                // Terminator frames carry no payload, skip encryption
                Vec::new()
            }
        };
        stage_times.add(Stage::Encrypt, start.elapsed());

        // ---- Stage 3: Frame header construction ----
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

    /// Runs the worker loop, processing frames until the channel closes or cancellation
    ///
    /// # Behavior
    /// - Spawns a new thread to process incoming frames
    /// - Checks cancellation flag before processing each frame
    /// - Sends encrypted frames to output channel
    /// - Propagates errors via fatal_tx to trigger pipeline shutdown
    /// - Exits gracefully when input channel closes or on cancellation
    pub fn run(
        self,
        rx: Receiver<FrameInput>,
        tx: Sender<Result<EncryptedFrame, FrameWorkerError>>,
    ) {
        // explicitly set DEBUG level
        tracing_logger(Some(tracing::Level::DEBUG));

        info!("[ENCRYPT FRAME WORKER] init, starting");
        // loop {
        //     // Check for cancellation before blocking on receive
        //     if self.cancelled.load(Ordering::Relaxed) {
        //         error!("[ENCRYPT FRAME WORKER] cancelled, exiting");
        //         break;
        //     }

        //     // Receive next frame input (blocks until available or channel closes)
        //     let input = match rx.recv() {
        //         Ok(input) => input,
        //         Err(_) => {
        //             // Channel closed normally - all frames processed
        //             debug!("[ENCRYPT FRAME WORKER] rx closed, exiting");
        //             break;
        //         }
        //     };

        //     // Process the frame
        //     match self.encrypt_frame(&input) {
        //         Ok(frame) => {
        //             // Send encrypted frame to output
        //             if let Err(e) = tx.send(Ok(frame)) {
        //                 // Output channel closed unexpectedly - pipeline is shutting down
        //                 error!("[ENCRYPT FRAME WORKER] tx send failed, receiver disconnected");
        //                 let _ = self.fatal_tx.send(StreamError::FrameWorker(
        //                     FrameWorkerError::StateError(e.to_string()),
        //                 ));
        //                 self.cancelled.store(true, Ordering::Relaxed);
        //                 break;
        //             }
        //         }
        //         Err(e) => {
        //             // Encryption failed - this is a fatal error
        //             error!("[ENCRYPT FRAME WORKER] encryption error: {:?}", e);

        //             // Try to send error to output (best effort)
        //             let _ = tx.send(Err(e.clone()));

        //             // Signal fatal error to pipeline monitor
        //             let _ = self.fatal_tx.send(StreamError::FrameWorker(e));

        //             // Set cancellation flag to stop other workers
        //             self.cancelled.store(true, Ordering::Relaxed);
        //             break;
        //         }
        //     }
        // }
        loop {
            crossbeam::select! {
                recv(rx) -> msg => {
                    match msg {
                        Ok(input) => {
                            // Check for cancellation before blocking on receive
                            if self.cancelled.load(Ordering::Relaxed) {
                                error!("[ENCRYPT FRAME WORKER] cancelled, exiting");
                                break;
                            }

                            // Process the frame
                            match self.encrypt_frame(&input) {
                                Ok(frame) => {
                                    // Send encrypted frame to output
                                    if let Err(e) = tx.send(Ok(frame)) {
                                        // Output channel closed unexpectedly - pipeline is shutting down
                                        error!("[ENCRYPT FRAME WORKER] tx send failed, receiver disconnected");
                                        let _ = self.fatal_tx.send(StreamError::FrameWorker(
                                            FrameWorkerError::StateError(e.to_string()),
                                        ));
                                        self.cancelled.store(true, Ordering::Relaxed);
                                        break;
                                    }
                                }
                                Err(e) => {
                                    // Encryption failed - this is a fatal error
                                    error!("[ENCRYPT FRAME WORKER] encryption error: {:?}", e);

                                    // Try to send error to output (best effort)
                                    let _ = tx.send(Err(e.clone()));

                                    // Signal fatal error to pipeline monitor
                                    let _ = self.fatal_tx.send(StreamError::FrameWorker(e));

                                    // Set cancellation flag to stop other workers
                                    self.cancelled.store(true, Ordering::Relaxed);
                                    break;
                                }
                            }
                        }
                        Err(_) => break, // channel closed
                    }
                }
                default(std::time::Duration::from_millis(10)) => {
                    // 🔥 THIS is the real cancellation path
                    if self.cancelled.load(Ordering::Relaxed) {
                        warn!("[ENCRYPT FRAME WORKER] cancelled (timeout path), exiting");
                        break;
                    }
                }
            }
        }

        warn!("[ENCRYPT FRAME WORKER] thread exiting");
    }

}

