use std::sync::Arc;

use async_trait::async_trait;

use argon2::{Argon2, PasswordHash, PasswordHasher as Argon2Hasher, PasswordVerifier};
use secrecy::ExposeSecret;
use tokio::sync::Semaphore;

use crate::{
    password::{PasswordHasher, errors::PasswordError},
    settings::get_settings,
};

#[derive(Debug)]
pub struct Argon2PasswordHasher {
    pepper: Arc<[u8]>,
    permits: Arc<Semaphore>,
}

impl Argon2PasswordHasher {
    pub fn new() -> Self {
        let settings = get_settings();
        let permits = Arc::new(Semaphore::new(settings.auth.concurrency_limit));
        Self {
            pepper: settings
                .auth
                .password_pepper
                .expose_secret()
                .as_bytes()
                .to_vec()
                .into(),
            permits,
        }
    }
}

#[async_trait]
impl PasswordHasher for Argon2PasswordHasher {
    async fn hash(&self, password: Vec<u8>) -> Result<String, PasswordError> {
        let _permit = self.permits.acquire().await?;
        let pepper = self.pepper.clone();

        let password_hash: String =
            tokio::task::spawn_blocking(move || -> Result<String, PasswordError> {
                let argon2 = Argon2::new_with_secret(
                    &pepper,
                    argon2::Algorithm::default(),
                    argon2::Version::default(),
                    argon2::Params::default(),
                )?;

                let password_hash = argon2
                    .hash_password(&password)
                    .map_err(PasswordError::from)?
                    .to_string();

                Ok(password_hash)
            })
            .await
            .map_err(PasswordError::from)??;

        Ok(password_hash)
    }

    async fn verify(&self, password: Vec<u8>, hash: &str) -> Result<bool, PasswordError> {
        let _permit = self.permits.acquire().await?;
        let pepper = self.pepper.clone();
        let hash = hash.to_string();

        let is_valid: bool = tokio::task::spawn_blocking(move || -> Result<bool, PasswordError> {
            let argon2 = Argon2::new_with_secret(
                &pepper,
                argon2::Algorithm::default(),
                argon2::Version::default(),
                argon2::Params::default(),
            )?;

            let hash = PasswordHash::new(&hash).map_err(PasswordError::from)?;
            Ok(argon2.verify_password(&password, &hash).is_ok())
        })
        .await
        .map_err(PasswordError::from)??;

        Ok(is_valid)
    }
}
