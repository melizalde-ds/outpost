use thiserror::Error;
use tonic::Status;
use uuid::Uuid;

use crate::repository::errors::RepositoryError;

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
    #[error("Unexpected error occurred: {0}")]
    UnexpectedError(#[from] anyhow::Error),
}

impl From<AuthServiceError> for Status {
    fn from(err: AuthServiceError) -> Self {
        match err {
            e @ (AuthServiceError::InvalidCredentials(_) | AuthServiceError::UserNotActive(_)) => {
                tracing::warn!(error = %e, "rejecting authentication");
                Status::unauthenticated("invalid credentials")
            }
            e @ AuthServiceError::UserAlreadyExists(_) => {
                tracing::trace!(error = %e, "username taken");
                Status::already_exists("username unavailable")
            }
            e @ AuthServiceError::UserNotFound(_) => {
                tracing::trace!(error = %e, "user not found");
                Status::not_found("user not found")
            }
            e @ (AuthServiceError::Repository(_) | AuthServiceError::UnexpectedError(_)) => {
                tracing::error!(error = ?e, "internal failure");
                Status::internal("internal error")
            }
        }
    }
}
