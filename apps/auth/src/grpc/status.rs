use tonic::Status;
use tracing::{error, info, warn};

use crate::{password::PasswordError, service::AuthServiceError};

impl From<AuthServiceError> for Status {
    fn from(err: AuthServiceError) -> Self {
        match err {
            e @ (AuthServiceError::InvalidCredentials(_) | AuthServiceError::UserNotActive(_)) => {
                info!(error = %e, "rejecting authentication");
                Status::unauthenticated("invalid credentials")
            }
            e @ AuthServiceError::UserAlreadyExists(_) => {
                info!(error = %e, "username taken");
                Status::already_exists("username unavailable")
            }
            e @ AuthServiceError::UserNotFound(_) => {
                info!(error = %e, "user not found");
                Status::not_found("user not found")
            }
            e @ AuthServiceError::Password(PasswordError::ShutDown(_)) => {
                warn!(error = %e, "password service shutting down");
                Status::unavailable("service unavailable, retry later")
            }
            e @ (AuthServiceError::Repository(_)
            | AuthServiceError::Password(_)
            | AuthServiceError::UnexpectedError(_)) => {
                error!(error = ?e, "internal failure");
                Status::internal("internal error")
            }
        }
    }
}
