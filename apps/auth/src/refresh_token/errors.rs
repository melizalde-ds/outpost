use thiserror::Error;

use crate::repository::{RepositoryError, TokenFamilyId};

#[derive(Error, Debug)]
pub enum RefreshTokenError {
    #[error("Invalid refresh token")]
    Invalid,
    #[error("Refresh token reused in family {0}")]
    Reused(TokenFamilyId),
    #[error("Redis command failed: {0}")]
    Command(#[from] redis::RedisError),
    #[error("Failed to serialize refresh token entry: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Failed to generate refresh token: {0}")]
    Random(#[from] getrandom::Error),
}
