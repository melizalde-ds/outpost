use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum SigningError {
    #[error("Key service call failed: {0}")]
    Call(#[from] tonic::Status),
    #[error("Key service returned an invalid key id: {0}")]
    BadKeyId(#[from] uuid::Error),
    #[error("Key {0}: expected 32 bytes, got {1}")]
    BadKeyLength(Uuid, usize),
    #[error("Key {0} is missing its expiration time")]
    MissingExpiry(Uuid),
    #[error("Key {0} has an invalid expiration time")]
    BadExpiry(Uuid),
}
