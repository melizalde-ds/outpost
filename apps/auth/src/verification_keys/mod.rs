use std::fmt::Debug;

use async_trait::async_trait;
use uuid::Uuid;

mod errors;
mod record;
mod redis;

pub use self::redis::RedisVerificationKeyStore;
pub use errors::{RecordError, VerificationKeyStoreError};
pub use record::JwkRecord;

#[async_trait]
pub trait VerificationKeyStore: Debug + Send + Sync + 'static {
    async fn has_verification_key(&self, kid: &Uuid) -> Result<bool, VerificationKeyStoreError>;
    async fn push_verification_key(&self, key: &JwkRecord)
    -> Result<(), VerificationKeyStoreError>;
    async fn push_multiple_verification_keys(
        &self,
        keys: &[JwkRecord],
    ) -> Result<(), VerificationKeyStoreError>;
}
