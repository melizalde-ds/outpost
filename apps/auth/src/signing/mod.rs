use std::fmt::Debug;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

mod errors;
mod key_service;

pub use errors::SigningError;
pub use key_service::WeatherSigningService;

#[async_trait]
pub trait SigningService: Debug + Send + Sync + 'static {
    async fn sign(&self, sub: Uuid) -> Result<SignedToken, SigningError>;
    async fn get_verification_keys(&self) -> Result<Vec<PublicKey>, SigningError>;
}

pub struct SignedToken {
    pub token: String,
    pub key_id: Uuid,
    pub new_key: Option<PublicKey>,
}

pub struct PublicKey {
    pub kid: Uuid,
    pub key: String,
    pub exp: DateTime<Utc>,
}
