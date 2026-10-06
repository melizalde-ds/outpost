use thiserror::Error;
use uuid::Uuid;

use crate::{
    password::PasswordError,
    redis::{RecordError, RedisClientError},
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
    #[error("Verification key record error: {0}")]
    Record(#[from] RecordError),
    #[error("Redis error: {0}")]
    Redis(#[from] RedisClientError),
    #[error("Unexpected error occurred: {0}")]
    UnexpectedError(#[from] anyhow::Error),
}
