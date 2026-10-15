use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};

use crate::{
    clock::{Clock, SystemClock},
    jwk_cache::JwkCache,
    password::PasswordHasher,
    refresh_token::RefreshTokenStore,
    repository::{TokenFamilyRepository, UserRepository},
    settings::RefreshTokenSettings,
    signing::SigningService,
};

mod auth;
mod errors;
mod refresh_token;

pub use errors::AuthServiceError;

pub struct Tokens {
    pub access_token: String,
    pub refresh_token: String,
    pub refresh_token_expires_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct DefaultAuthService {
    users: Arc<dyn UserRepository>,
    families: Arc<dyn TokenFamilyRepository>,
    passwords: Arc<dyn PasswordHasher>,
    signer: Arc<dyn SigningService>,
    jwk_cache: Arc<dyn JwkCache>,
    refresh_tokens: Arc<dyn RefreshTokenStore>,
    clock: Arc<dyn Clock>,
    idle_timeout: Duration,
    max_age: Duration,
}

impl DefaultAuthService {
    pub fn new(
        users: Arc<dyn UserRepository>,
        families: Arc<dyn TokenFamilyRepository>,
        passwords: Arc<dyn PasswordHasher>,
        signer: Arc<dyn SigningService>,
        jwk_cache: Arc<dyn JwkCache>,
        refresh_tokens: Arc<dyn RefreshTokenStore>,
        settings: &RefreshTokenSettings,
    ) -> Self {
        Self {
            users,
            families,
            passwords,
            signer,
            jwk_cache,
            refresh_tokens,
            clock: Arc::new(SystemClock),
            idle_timeout: Duration::seconds(settings.idle_timeout_secs as i64),
            max_age: Duration::seconds(settings.max_age_secs as i64),
        }
    }
    #[cfg(test)]
    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }
}
