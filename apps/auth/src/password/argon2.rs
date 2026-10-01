use std::sync::Arc;

use async_trait::async_trait;

use argon2::{Argon2, PasswordHash, PasswordHasher as Argon2Hasher, PasswordVerifier};
use secrecy::{ExposeSecret, SecretSlice, SecretString};
use tokio::sync::Semaphore;

use crate::{
    password::{PasswordHasher, errors::PasswordError},
    settings::password::PasswordSettings,
};

#[derive(Debug)]
pub struct Argon2PasswordHasher {
    pepper: Arc<SecretString>,
    permits: Arc<Semaphore>,
}

impl Argon2PasswordHasher {
    pub fn new(settings: &PasswordSettings) -> Self {
        Self {
            pepper: Arc::new(settings.pepper.expose_secret().into()),
            permits: Arc::new(Semaphore::new(settings.concurrency_limit)),
        }
    }
}

#[async_trait]
impl PasswordHasher for Argon2PasswordHasher {
    async fn hash(&self, password: SecretSlice<u8>) -> Result<String, PasswordError> {
        let _permit = self.permits.acquire().await?;
        let pepper = self.pepper.clone();

        let password_hash: String =
            tokio::task::spawn_blocking(move || -> Result<String, PasswordError> {
                let argon2 = Argon2::new_with_secret(
                    pepper.expose_secret().as_bytes(),
                    argon2::Algorithm::default(),
                    argon2::Version::default(),
                    argon2::Params::default(),
                )?;

                let password_hash = argon2
                    .hash_password(password.expose_secret())
                    .map_err(PasswordError::from)?
                    .to_string();

                Ok(password_hash)
            })
            .await
            .map_err(PasswordError::from)??;

        Ok(password_hash)
    }

    async fn verify(&self, password: SecretSlice<u8>, hash: String) -> Result<bool, PasswordError> {
        let _permit = self.permits.acquire().await?;
        let pepper = self.pepper.clone();

        let is_valid: bool = tokio::task::spawn_blocking(move || -> Result<bool, PasswordError> {
            let argon2 = Argon2::new_with_secret(
                pepper.expose_secret().as_bytes(),
                argon2::Algorithm::default(),
                argon2::Version::default(),
                argon2::Params::default(),
            )?;

            let hash = PasswordHash::new(&hash).map_err(PasswordError::from)?;
            Ok(argon2
                .verify_password(password.expose_secret(), &hash)
                .is_ok())
        })
        .await
        .map_err(PasswordError::from)??;

        Ok(is_valid)
    }
}
