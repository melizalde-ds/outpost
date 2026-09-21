use std::sync::Arc;

use async_trait::async_trait;

use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use tokio::sync::Semaphore;

use crate::password::{PasswordService, errors::PasswordError};

#[derive(Debug)]
pub struct Argon2PasswordService {
    pepper: Arc<[u8]>,
    permits: Arc<Semaphore>,
}

impl Argon2PasswordService {
    pub fn new(pepper: Vec<u8>, max_concurrent: usize) -> Self {
        let permits = Arc::new(Semaphore::new(max_concurrent));
        Self {
            pepper: pepper.into(),
            permits,
        }
    }
}

#[async_trait]
impl PasswordService for Argon2PasswordService {
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
