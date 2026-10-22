use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};

use crate::{
    clock::{Clock, SystemClock},
    jwk_cache::{JwkCache, JwkRecord, LocalJwkSet},
    password::PasswordHasher,
    refresh_token::RefreshTokenStore,
    repository::{TokenFamilyRepository, UserRepository},
    settings::RefreshTokenSettings,
    signing::SigningService,
};

mod auth;
mod errors;
mod jwks;
mod refresh_token;

pub use errors::AuthServiceError;

// Serialized as-is by the HTTP API; the field names are part of its JSON contract.
#[derive(serde::Serialize)]
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
    local_jwks: Arc<LocalJwkSet>,
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
            local_jwks: Arc::default(),
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
    #[cfg(test)]
    pub fn with_local_jwks(mut self, local_jwks: Arc<LocalJwkSet>) -> Self {
        self.local_jwks = local_jwks;
        self
    }

    pub(crate) fn live_jwks(&self) -> Option<Vec<JwkRecord>> {
        self.local_jwks.live(self.clock.now())
    }
}
