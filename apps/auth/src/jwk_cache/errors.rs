use thiserror::Error;

#[derive(Error, Debug)]
pub enum JwkCacheError {
    #[error("Redis command failed: {0}")]
    Command(#[from] redis::RedisError),
}

#[derive(Error, Debug)]
pub enum JwkRecordError {
    #[error("Failed to serialize JWK: {0}")]
    Serialize(#[from] serde_json::Error),
}
