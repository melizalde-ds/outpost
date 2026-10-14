use std::fmt::Debug;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::repository::UserId;

mod errors;
mod redis;
mod token;

pub use self::redis::RedisRefreshTokenStore;
pub use errors::RefreshTokenStoreError;
pub use token::RefreshToken;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Entry {
    Live {
        user_id: UserId,
        family_exp: DateTime<Utc>,
    },
    Rotated,
}

#[async_trait]
pub trait RefreshTokenStore: Debug + Send + Sync + 'static {
    async fn insert(&self, token: &RefreshToken) -> Result<(), RefreshTokenStoreError>;
    async fn find(&self, hash: &[u8; 32]) -> Result<Option<Entry>, RefreshTokenStoreError>;
    async fn rotate(
        &self,
        old: &[u8; 32],
        new: &RefreshToken,
    ) -> Result<Option<Entry>, RefreshTokenStoreError>;
    async fn revoke(&self, hash: &[u8; 32]) -> Result<(), RefreshTokenStoreError>;
}
