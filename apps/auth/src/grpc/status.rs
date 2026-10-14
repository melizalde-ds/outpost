use tonic::{Code, Status};
use tracing::{error, info, warn};

use crate::{
    password::PasswordError, refresh_token::RefreshTokenStoreError, service::AuthServiceError,
    signing::SigningError,
};

impl From<AuthServiceError> for Status {
    fn from(err: AuthServiceError) -> Self {
        match err {
            e @ (AuthServiceError::UserNotFound(_)
            | AuthServiceError::InvalidCredentials(_)
            | AuthServiceError::UserNotActive(_)
            | AuthServiceError::InvalidRefreshToken) => {
                info!(error = %e, "rejecting authentication");
                Status::unauthenticated("invalid credentials")
            }
            AuthServiceError::RefreshTokenReused(family_id) => {
                warn!(%family_id, "refresh token reuse detected");
                Status::unauthenticated("invalid credentials")
            }
            e @ AuthServiceError::UserAlreadyExists(_) => {
                info!(error = %e, "username taken");
                Status::already_exists("username unavailable")
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
            e @ AuthServiceError::RefreshTokenStore(RefreshTokenStoreError::Command(_)) => {
                warn!(error = %e, "refresh token store unavailable");
                Status::unavailable("service unavailable, retry later")
            }
            e @ (AuthServiceError::Repository(_)
            | AuthServiceError::RefreshTokenStore(_)
            | AuthServiceError::Random(_)
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
