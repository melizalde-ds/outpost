use thiserror::Error;

#[derive(Error, Debug)]
pub enum RefreshTokenStoreError {
    #[error("Redis command failed: {0}")]
    Command(#[from] redis::RedisError),
}
