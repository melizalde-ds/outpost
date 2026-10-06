use thiserror::Error;

#[derive(Error, Debug)]
pub enum RedisClientError {
    #[error("Redis command failed: {0}")]
    Command(#[from] redis::RedisError),
}

#[derive(Error, Debug)]
pub enum RecordError {
    #[error("Failed to serialize JWK: {0}")]
    Serialize(#[from] serde_json::Error),
}
