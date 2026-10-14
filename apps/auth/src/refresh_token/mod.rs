use std::fmt::Debug;

use async_trait::async_trait;
use uuid::Uuid;

mod errors;
mod redis;
mod token;

pub use self::redis::RedisRefreshTokenStore;
pub use errors::RefreshTokenStoreError;
pub use token::RefreshToken;

#[async_trait]
pub trait RefreshTokenStore: Debug + Send + Sync + 'static {
    async fn insert(&self, token: &RefreshToken) -> Result<(), RefreshTokenStoreError>;
    async fn find(&self, hash: &[u8; 32]) -> Result<Option<Uuid>, RefreshTokenStoreError>;
    async fn revoke(&self, hash: &[u8; 32]) -> Result<(), RefreshTokenStoreError>;
}
