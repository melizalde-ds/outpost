use thiserror::Error;

#[derive(Error, Debug)]
pub enum RefreshTokenStoreError {
    #[error("Redis command failed: {0}")]
    Command(#[from] redis::RedisError),
}

#[derive(Error, Debug)]
pub enum RefreshTokenError {
    #[error("Refresh token has an invalid id: {0}")]
    BadId(#[from] uuid::Error),
}
