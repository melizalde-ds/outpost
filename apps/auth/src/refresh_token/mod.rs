use std::fmt::Debug;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

mod errors;
mod redis;

pub use self::redis::RedisRefreshTokenStore;
pub use errors::RefreshTokenStoreError;

#[async_trait]
pub trait RefreshTokenStore: Debug + Send + Sync + 'static {
    async fn insert(
        &self,
        id: &Uuid,
        user_id: &Uuid,
        exp: DateTime<Utc>,
    ) -> Result<(), RefreshTokenStoreError>;
    async fn find(&self, id: &Uuid) -> Result<Option<Uuid>, RefreshTokenStoreError>;
    async fn revoke(&self, id: &Uuid) -> Result<(), RefreshTokenStoreError>;
}
