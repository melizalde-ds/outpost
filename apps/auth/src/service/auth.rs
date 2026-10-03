use std::sync::Arc;

use secrecy::SecretSlice;
use uuid::Uuid;

use crate::{
    cache::JwkCache,
    password::PasswordHasher,
    proto::auth::v1::{
        LoginRequest, LoginResponse, RefreshTokenRequest, RefreshTokenResponse, RegisterRequest,
        RegisterResponse, TokenPair,
    },
    repository::{AuthRepository, RepositoryError},
    service::AuthServiceError,
    signing::SigningService,
};

#[derive(Debug)]
pub struct WeatherAuthService {
    repo: Arc<dyn AuthRepository>,
    passwords: Arc<dyn PasswordHasher>,
    signing_service: Arc<dyn SigningService>,
    key_cache: JwkCache,
}

impl WeatherAuthService {
    pub fn new(
        repo: Arc<dyn AuthRepository>,
        passwords: Arc<dyn PasswordHasher>,
        signing_service: Arc<dyn SigningService>,
        key_cache: JwkCache,
    ) -> Self {
        Self {
            repo,
            passwords,
            signing_service,
            key_cache,
        }
    }

    pub(crate) async fn register(
        &self,
        req: RegisterRequest,
    ) -> Result<RegisterResponse, AuthServiceError> {
        let password = SecretSlice::from(req.password.into_bytes());
        let password_hash = self.passwords.hash(password).await?;

        match self
            .repo
            .insert_credentials(&req.username, password_hash)
            .await
        {
            Ok(user_id) => {
                let token = self.issue_token(user_id.0).await?;

                Ok(RegisterResponse {
                    tokens: Some(TokenPair {
                        access_token: token,
                        ..Default::default()
                    }),
                })
            }
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

        let password = SecretSlice::from(req.password.into_bytes());
        let password_matches = self
            .passwords
            .verify(password, credentials.password_hash)
            .await?;

        if !credentials.active {
            return Err(AuthServiceError::UserNotActive(credentials.user_id));
        }

        match password_matches {
            true => {
                let token = self.issue_token(credentials.user_id).await?;

                Ok(LoginResponse {
                    tokens: Some(TokenPair {
                        access_token: token,
                        ..Default::default()
                    }),
                })
            }
            false => Err(AuthServiceError::InvalidCredentials(req.username)),
        }
    }

    async fn issue_token(&self, user_id: Uuid) -> Result<String, AuthServiceError> {
        let signed = self.signing_service.sign(user_id).await?;

        if let Some(key) = signed.new_key {
            self.key_cache.insert(key)?;
        }

        if !self.key_cache.contains(&signed.key_id) {
            let keys = self.signing_service.get_verification_keys().await?;
            self.key_cache.rebuild(keys)?;
        }

        Ok(signed.token)
    }

    pub(crate) async fn refresh_token(
        &self,
        _req: RefreshTokenRequest,
    ) -> Result<RefreshTokenResponse, AuthServiceError> {
        let user_id = Uuid::now_v7();

        if !self.repo.is_active(&user_id).await? {
            return Err(AuthServiceError::UserNotActive(user_id));
        }

        Ok(RefreshTokenResponse::default())
    }
}
