use std::sync::Arc;

use async_trait::async_trait;

use argon2::{Argon2, Params, PasswordHash, PasswordHasher as Argon2Hasher, PasswordVerifier};
use secrecy::{ExposeSecret, SecretSlice, SecretString};
use tokio::sync::Semaphore;

use crate::{
    password::{PasswordError, PasswordHasher},
    settings::PasswordSettings,
};

#[derive(Debug)]
pub struct Argon2PasswordHasher {
    pepper: Arc<SecretString>,
    permits: Arc<Semaphore>,
    params: Params,
    dummy_hash: String,
}

impl Argon2PasswordHasher {
    pub fn new(settings: &PasswordSettings) -> Result<Self, PasswordError> {
        Self::with_params(settings, Params::default())
    }

    pub fn with_params(settings: &PasswordSettings, params: Params) -> Result<Self, PasswordError> {
        let pepper: SecretString = settings.pepper.expose_secret().into();
        let dummy_hash = Argon2::new_with_secret(
            pepper.expose_secret().as_bytes(),
            argon2::Algorithm::default(),
            argon2::Version::default(),
            params.clone(),
        )?
        .hash_password(b"dummy password")?
        .to_string();

        Ok(Self {
            pepper: Arc::new(pepper),
            permits: Arc::new(Semaphore::new(settings.concurrency_limit)),
            params,
            dummy_hash,
        })
    }
}

#[async_trait]
impl PasswordHasher for Argon2PasswordHasher {
    async fn hash(&self, password: SecretSlice<u8>) -> Result<String, PasswordError> {
        let _permit = self.permits.acquire().await?;
        let pepper = self.pepper.clone();
        let params = self.params.clone();

        let password_hash: String =
            tokio::task::spawn_blocking(move || -> Result<String, PasswordError> {
                let argon2 = Argon2::new_with_secret(
                    pepper.expose_secret().as_bytes(),
                    argon2::Algorithm::default(),
                    argon2::Version::default(),
                    params,
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
        let params = self.params.clone();

        let is_valid: bool = tokio::task::spawn_blocking(move || -> Result<bool, PasswordError> {
            let argon2 = Argon2::new_with_secret(
                pepper.expose_secret().as_bytes(),
                argon2::Algorithm::default(),
                argon2::Version::default(),
                params,
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

    async fn verify_dummy(&self, password: SecretSlice<u8>) -> Result<(), PasswordError> {
        self.verify(password, self.dummy_hash.clone()).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PASSWORD: &str = "correct horse";

    fn hasher(pepper: &str) -> Argon2PasswordHasher {
        let settings = PasswordSettings {
            pepper: SecretString::from(pepper),
            concurrency_limit: 1,
        };
        let cheapest = Params::new(8, 1, 1, None).unwrap();
        Argon2PasswordHasher::with_params(&settings, cheapest).unwrap()
    }

    fn secret(password: &str) -> SecretSlice<u8> {
        SecretSlice::from(password.as_bytes().to_vec())
    }

    #[tokio::test]
    async fn verify_right_password_returns_true() {
        // Arrange
        let hasher = hasher("pepper");
        let hash = hasher.hash(secret(PASSWORD)).await.unwrap();

        // Act
        let matches = hasher.verify(secret(PASSWORD), hash).await.unwrap();

        // Assert
        assert!(matches);
    }

    #[tokio::test]
    async fn verify_wrong_password_returns_false() {
        // Arrange
        let hasher = hasher("pepper");
        let hash = hasher.hash(secret(PASSWORD)).await.unwrap();

        // Act
        let matches = hasher.verify(secret("wrong password"), hash).await.unwrap();

        // Assert
        assert!(!matches);
    }

    #[tokio::test]
    async fn verify_with_different_pepper_returns_false() {
        // Arrange
        let hash = hasher("pepper").hash(secret(PASSWORD)).await.unwrap();

        // Act
        let matches = hasher("other pepper")
            .verify(secret(PASSWORD), hash)
            .await
            .unwrap();

        // Assert
        assert!(!matches);
    }

    #[tokio::test]
    async fn verify_dummy_any_password_completes() {
        // Arrange
        let hasher = hasher("pepper");

        // Act
        let result = hasher.verify_dummy(secret(PASSWORD)).await;

        // Assert
        assert!(result.is_ok());
    }
}
