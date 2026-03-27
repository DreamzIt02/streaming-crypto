
// ## 2️⃣ `core.rs` — stable public API

use std::ops::Deref;

use crate::{
    constants::MASTER_KEY_LENGTHS, 
    crypto::CryptoError, 
    types::StreamError
};

#[derive(Clone)]
pub struct MasterKey(Vec<u8>);

impl Deref for MasterKey {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl MasterKey {
    pub fn new(bytes: Vec<u8>) -> Self {
        // let len = bytes.len();

        // if MASTER_KEY_LENGTHS.contains(&len) {
        //     Ok(Self(bytes))
        // } else {
        //     Err(StreamError::Crypto(
        //         CryptoError::InvalidKeyLen {
        //             expected: &MASTER_KEY_LENGTHS,
        //             actual: len,
        //         },
        //     ))
        // }
        Self(bytes)
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    /// Validate that the provided bytes match one of the allowed key lengths.
    pub fn validate(bytes: &[u8]) -> Result<(), StreamError> {
        let len = bytes.len();

        if MASTER_KEY_LENGTHS.contains(&len) {
            Ok(())
        } else {
            Err(StreamError::Crypto(
                CryptoError::InvalidKeyLen {
                    expected: &MASTER_KEY_LENGTHS,
                    actual: len,
                },
            ))
        }
    }
}
