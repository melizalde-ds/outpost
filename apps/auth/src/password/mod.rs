use std::fmt::Debug;

use async_trait::async_trait;
use secrecy::SecretSlice;

mod argon2;
mod errors;

pub use self::argon2::Argon2PasswordHasher;
pub use errors::PasswordError;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait PasswordHasher: Debug + Send + Sync + 'static {
    async fn hash(&self, password: SecretSlice<u8>) -> Result<String, PasswordError>;
    async fn verify(&self, password: SecretSlice<u8>, hash: String) -> Result<bool, PasswordError>;
    async fn verify_dummy(&self, password: SecretSlice<u8>) -> Result<(), PasswordError>;
}
