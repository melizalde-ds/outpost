use thiserror::Error;

#[derive(Error, Debug)]
pub enum RefreshTokenStoreError {
    #[error("Redis command failed: {0}")]
    Command(#[from] redis::RedisError),
    #[error("Failed to serialize refresh token entry: {0}")]
    Serialize(#[from] serde_json::Error),
}
