use std::sync::Arc;

use chrono::{DateTime, Utc};
use prost_types::Timestamp;
use secrecy::SecretSlice;
use uuid::Uuid;

use crate::{
    jwk_cache::{JwkCache, JwkRecord},
    password::PasswordHasher,
    proto::auth::v1::{
        LoginRequest, LoginResponse, RefreshTokenRequest, RefreshTokenResponse, RegisterRequest,
        RegisterResponse, TokenPair,
    },
    refresh_token::RefreshTokenService,
    repository::{RepositoryError, RevocationReason, UserRepository},
    service::AuthServiceError,
    signing::SigningService,
};

#[derive(Debug)]
pub struct DefaultAuthService {
    user_repo: Arc<dyn UserRepository>,
    passwords: Arc<dyn PasswordHasher>,
    signing_service: Arc<dyn SigningService>,
    jwk_cache: Arc<dyn JwkCache>,
    refresh_tokens: Arc<dyn RefreshTokenService>,
}

impl DefaultAuthService {
    pub fn new(
        user_repo: Arc<dyn UserRepository>,
        passwords: Arc<dyn PasswordHasher>,
        signing_service: Arc<dyn SigningService>,
        jwk_cache: Arc<dyn JwkCache>,
        refresh_tokens: Arc<dyn RefreshTokenService>,
    ) -> Self {
        Self {
            user_repo,
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
            .user_repo
            .insert_credentials(&req.username, password_hash)
            .await
        {
            Ok(user_id) => {
                let token = self.issue_token(user_id.0).await?;
                let refresh = self.refresh_tokens.issue(user_id).await?;

                Ok(RegisterResponse {
                    tokens: Some(TokenPair {
                        access_token: token,
                        refresh_token: refresh.token,
                        refresh_token_expires_at: Some(to_timestamp(refresh.exp)),
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
        let Some(credentials) = self.user_repo.get_credentials(&req.username).await? else {
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
                let refresh = self
                    .refresh_tokens
                    .issue(credentials.user_id.into())
                    .await?;

                Ok(LoginResponse {
                    tokens: Some(TokenPair {
                        access_token: token,
                        refresh_token: refresh.token,
                        refresh_token_expires_at: Some(to_timestamp(refresh.exp)),
                    }),
                })
            }
            false => Err(AuthServiceError::InvalidCredentials(req.username)),
        }
    }

    pub(crate) async fn refresh_token(
        &self,
        req: RefreshTokenRequest,
    ) -> Result<RefreshTokenResponse, AuthServiceError> {
        let session = self.refresh_tokens.validate(&req.refresh_token).await?;

        if !self.user_repo.is_active(&session.user_id.0).await? {
            self.refresh_tokens
                .revoke(&session, RevocationReason::Inactive)
                .await?;
            return Err(AuthServiceError::UserNotActive(session.user_id.0));
        }

        let token = self.issue_token(session.user_id.0).await?;
        let refresh = self.refresh_tokens.rotate(&session).await?;

        Ok(RefreshTokenResponse {
            tokens: Some(TokenPair {
                access_token: token,
                refresh_token: refresh.token,
                refresh_token_expires_at: Some(to_timestamp(refresh.exp)),
            }),
        })
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
}

fn to_timestamp(dt: DateTime<Utc>) -> Timestamp {
    Timestamp {
        seconds: dt.timestamp(),
        nanos: dt.timestamp_subsec_nanos() as i32,
    }
}
