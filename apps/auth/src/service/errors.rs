use thiserror::Error;
use uuid::Uuid;

use crate::{
    jwk_cache::{JwkCacheError, JwkRecordError},
    password::PasswordError,
    refresh_token::RefreshTokenError,
    repository::RepositoryError,
    signing::SigningError,
};

#[derive(Error, Debug)]
pub enum AuthServiceError {
    #[error("User {0} not found")]
    UserNotFound(String),
    #[error("Invalid credentials provided for user {0}")]
    InvalidCredentials(String),
    #[error("User with ID {0} is not active")]
    UserNotActive(Uuid),
    #[error("Username {0} already exists")]
    UserAlreadyExists(String),
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Password engine error: {0}")]
    Password(#[from] PasswordError),
    #[error("Signing error: {0}")]
    Signing(#[from] SigningError),
    #[error("JWK record error: {0}")]
    JwkRecord(#[from] JwkRecordError),
    #[error("JWK cache error: {0}")]
    JwkCache(#[from] JwkCacheError),
    #[error("Refresh token error: {0}")]
    RefreshToken(#[from] RefreshTokenError),
    #[error("Unexpected error occurred: {0}")]
    UnexpectedError(#[from] anyhow::Error),
}
