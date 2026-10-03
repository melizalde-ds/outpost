use std::fmt::Debug;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

mod key_service;

pub use key_service::WeatherSigningService;

#[async_trait]
pub trait SigningService: Debug + Send + Sync + 'static {
    async fn sign(&self, sub: Uuid) -> anyhow::Result<SignedToken>;
    async fn get_verification_keys(&self) -> anyhow::Result<Vec<PublicKey>>;
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
