use thiserror::Error;

#[derive(Error, Debug)]
pub enum VerificationKeyStoreError {
    #[error("Redis command failed: {0}")]
    Command(#[from] redis::RedisError),
}

#[derive(Error, Debug)]
pub enum RecordError {
    #[error("Failed to serialize JWK: {0}")]
    Serialize(#[from] serde_json::Error),
}
