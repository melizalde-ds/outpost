use std::fmt::Debug;

use async_trait::async_trait;

use crate::password::errors::PasswordError;

pub(crate) mod argon2;
pub(crate) mod errors;

#[async_trait]
pub trait PasswordService: Debug + Send + Sync + 'static {
    async fn hash(&self, password: Vec<u8>) -> Result<String, PasswordError>;
    async fn verify(&self, password: Vec<u8>, hash: &str) -> Result<bool, PasswordError>;
}
