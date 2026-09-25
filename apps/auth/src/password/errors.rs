use argon2::password_hash::Error as HashError;
use argon2::password_hash::phc::Error as PhcError;
use thiserror::Error;
use tokio::{sync::AcquireError, task::JoinError};

#[derive(Error, Debug)]
pub enum PasswordError {
    #[error("Password service is shutting down: {0}")]
    ShutDown(#[from] AcquireError),
    #[error("Invalid argon2 configuration: {0}")]
    Config(#[from] argon2::Error),
    #[error("Password hashing task did not complete: {0}")]
    Join(#[from] JoinError),
    #[error("Password hashing failed: {0}")]
    Hash(#[from] HashError),
    #[error("Stored password hash is malformed or unsupported: {0}")]
    CorruptHash(#[from] PhcError),
}
