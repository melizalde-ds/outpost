use std::fmt::Debug;

use async_trait::async_trait;
use secrecy::SecretSlice;

use crate::password::errors::PasswordError;

pub(crate) mod argon2;
pub(crate) mod errors;

#[async_trait]
pub trait PasswordHasher: Debug + Send + Sync + 'static {
    async fn hash(&self, password: SecretSlice<u8>) -> Result<String, PasswordError>;
    async fn verify(&self, password: SecretSlice<u8>, hash: String) -> Result<bool, PasswordError>;
}
