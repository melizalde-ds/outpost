use std::fmt::Debug;

use async_trait::async_trait;
use uuid::Uuid;

mod errors;
mod record;
mod redis;

pub use self::redis::RedisJwkCache;
pub use errors::{JwkCacheError, JwkRecordError};
pub use record::JwkRecord;

#[async_trait]
pub trait JwkCache: Debug + Send + Sync + 'static {
    async fn contains(&self, kid: &Uuid) -> Result<bool, JwkCacheError>;
    async fn insert(&self, key: &JwkRecord) -> Result<(), JwkCacheError>;
    async fn insert_many(&self, keys: &[JwkRecord]) -> Result<(), JwkCacheError>;
}
