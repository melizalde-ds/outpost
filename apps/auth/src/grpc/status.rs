use tonic::{Code, Status};
use tracing::{error, info, warn};

use crate::{password::PasswordError, service::AuthServiceError, signing::SigningError};

impl From<AuthServiceError> for Status {
    fn from(err: AuthServiceError) -> Self {
        match err {
            e @ (AuthServiceError::InvalidCredentials(_)
            | AuthServiceError::UserNotActive(_)
            | AuthServiceError::InvalidRefreshToken) => {
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
            AuthServiceError::Signing(SigningError::Call(status))
                if matches!(status.code(), Code::Unavailable | Code::DeadlineExceeded) =>
            {
                warn!(error = %status, "key service unavailable");
                Status::unavailable("service unavailable, retry later")
            }
            e @ AuthServiceError::JwkCache(_) => {
                warn!(error = %e, "JWK cache unavailable");
                Status::unavailable("service unavailable, retry later")
            }
            e @ AuthServiceError::RefreshTokenStore(_) => {
                warn!(error = %e, "refresh token store unavailable");
                Status::unavailable("service unavailable, retry later")
            }
            e @ (AuthServiceError::Repository(_)
            | AuthServiceError::Password(_)
            | AuthServiceError::Signing(_)
            | AuthServiceError::JwkRecord(_)
            | AuthServiceError::UnexpectedError(_)) => {
                error!(error = ?e, "internal failure");
                Status::internal("internal error")
            }
        }
    }
}
