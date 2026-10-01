use std::fmt::Debug;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

pub(crate) mod auth;
pub(crate) mod errors;
pub(crate) mod signing;

#[async_trait]
pub trait SigningService: Debug + Send + Sync + 'static {
    fn sign(&self, sub: Uuid) -> anyhow::Result<String>;
    fn get_verification_keys(&self) -> anyhow::Result<Vec<PublicKey>>;
}

pub struct PublicKey {
    pub kid: Uuid,
    pub key: String,
    pub exp: DateTime<Utc>,
}
