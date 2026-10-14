use std::sync::Arc;

use secrecy::SecretSlice;
use uuid::Uuid;

use crate::{
    jwk_cache::{JwkCache, JwkRecord},
    password::PasswordHasher,
    proto::auth::v1::{
        LoginRequest, LoginResponse, RefreshTokenRequest, RefreshTokenResponse, RegisterRequest,
        RegisterResponse, TokenPair,
    },
    refresh_token::RefreshTokenStore,
    repository::{AuthRepository, RepositoryError},
    service::AuthServiceError,
    signing::SigningService,
};

#[derive(Debug)]
pub struct DefaultAuthService {
    repo: Arc<dyn AuthRepository>,
    passwords: Arc<dyn PasswordHasher>,
    signing_service: Arc<dyn SigningService>,
    jwk_cache: Arc<dyn JwkCache>,
    refresh_tokens: Arc<dyn RefreshTokenStore>,
}

impl DefaultAuthService {
    pub fn new(
        repo: Arc<dyn AuthRepository>,
        passwords: Arc<dyn PasswordHasher>,
        signing_service: Arc<dyn SigningService>,
        jwk_cache: Arc<dyn JwkCache>,
        refresh_tokens: Arc<dyn RefreshTokenStore>,
    ) -> Self {
        Self {
            repo,
            passwords,
            signing_service,
            jwk_cache,
            refresh_tokens,
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
            let record = JwkRecord::try_from(key)?;
            self.jwk_cache.insert(&record).await?;
        } else if !self.jwk_cache.contains(&signed.key_id).await? {
            let records = self
                .signing_service
                .get_verification_keys()
                .await?
                .into_iter()
                .map(JwkRecord::try_from)
                .collect::<Result<Vec<_>, _>>()?;
            self.jwk_cache.insert_many(&records).await?;
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
