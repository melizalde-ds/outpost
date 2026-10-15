use thiserror::Error;

use crate::{
    jwk_cache::{JwkCacheError, JwkRecordError},
    password::PasswordError,
    refresh_token::RefreshTokenStoreError,
    repository::{RepositoryError, TokenFamilyId, UserId},
    signing::SigningError,
};

#[derive(Error, Debug)]
pub enum AuthServiceError {
    #[error("User {0} not found")]
    UserNotFound(String),
    #[error("Invalid credentials provided for user {0}")]
    InvalidCredentials(String),
    #[error("User with ID {0} is not active")]
    UserNotActive(UserId),
    #[error("Username {0} already exists")]
    UserAlreadyExists(String),
    #[error("Invalid refresh token")]
    InvalidRefreshToken,
    #[error("Account {0} was created, but signing in failed: {1}")]
    SignInAfterRegister(UserId, #[source] Box<AuthServiceError>),
    #[error("Refresh token reused in family {0}")]
    RefreshTokenReused(TokenFamilyId),
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
    #[error("Refresh token store error: {0}")]
    RefreshTokenStore(#[from] RefreshTokenStoreError),
    #[error("Random number generation failed: {0}")]
    Random(#[from] getrandom::Error),
    #[error("Unexpected error occurred: {0}")]
    UnexpectedError(#[from] anyhow::Error),
}
