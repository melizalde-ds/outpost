use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum CacheError {
    #[error("Cannot insert expired JWK record with kid {0}")]
    ExpiredKey(Uuid),
    #[error("Failed to serialize JWK: {0}")]
    Serialize(#[from] serde_json::Error),
}
