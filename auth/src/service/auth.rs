use std::sync::Arc;

use uuid::Uuid;

use crate::{
    password::PasswordService,
    proto::auth::v1::{
        LoginRequest, LoginResponse, RefreshTokenRequest, RefreshTokenResponse, RegisterRequest,
        RegisterResponse,
    },
    repository::{AuthRepository, errors::RepositoryError},
    service::errors::AuthServiceError,
};

#[derive(Debug)]
pub struct WeatherAuthService {
    repo: Arc<dyn AuthRepository>,
    passwords: Arc<dyn PasswordService>,
}

impl WeatherAuthService {
    pub fn new(repo: Arc<dyn AuthRepository>, passwords: Arc<dyn PasswordService>) -> Self {
        Self { repo, passwords }
    }

    pub(crate) async fn register(
        &self,
        req: RegisterRequest,
    ) -> Result<RegisterResponse, AuthServiceError> {
        let password_hash = self.passwords.hash(req.password.into_bytes()).await?;

        match self
            .repo
            .insert_credentials(&req.username, password_hash)
            .await
        {
            Ok(user_id) => Ok(RegisterResponse {
                user_id: user_id.0.to_string(),
                ..Default::default()
            }),
            Err(RepositoryError::UniqueViolation(_)) => {
                Err(AuthServiceError::UserAlreadyExists(req.username))
            }
            Err(e) => Err(e.into()),
        }
    }

    pub(crate) async fn login(&self, req: LoginRequest) -> Result<LoginResponse, AuthServiceError> {
        let Some(credentials) = self.repo.get_credentials(&req.username).await? else {
            return Err(AuthServiceError::UserNotFound(req.username));
        };

        if !credentials.active {
            return Err(AuthServiceError::UserNotActive(credentials.user_id));
        }

        let password_matches = self
            .passwords
            .verify(req.password.into_bytes(), &credentials.password_hash)
            .await?;

        match password_matches {
            true => Ok(LoginResponse::default()),
            false => Err(AuthServiceError::InvalidCredentials(req.username)),
        }
    }

    pub(crate) async fn refresh_token(
        &self,
        _req: RefreshTokenRequest,
    ) -> Result<RefreshTokenResponse, AuthServiceError> {
        let user_id = Uuid::new_v4();

        if !self.repo.is_active(&user_id).await? {
            return Err(AuthServiceError::UserNotActive(user_id));
        }

        Ok(RefreshTokenResponse::default())
    }
}
