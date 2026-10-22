use std::fmt::Debug;

use async_trait::async_trait;
use uuid::Uuid;

mod errors;
mod local;
mod record;
mod redis;

pub use self::redis::RedisJwkCache;
pub use errors::{JwkCacheError, JwkRecordError};
pub use local::LocalJwkSet;
pub use record::JwkRecord;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait JwkCache: Debug + Send + Sync + 'static {
    async fn contains(&self, kid: &Uuid) -> Result<bool, JwkCacheError>;
    async fn insert(&self, key: &JwkRecord) -> Result<(), JwkCacheError>;
    async fn insert_many(&self, keys: &[JwkRecord]) -> Result<(), JwkCacheError>;
}
