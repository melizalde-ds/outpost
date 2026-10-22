use secrecy::{ExposeSecret, SecretSlice};

use crate::{
    jwk_cache::JwkRecord,
    repository::{RepositoryError, UserId},
    service::{AuthServiceError, DefaultAuthService, Tokens},
};

const MAX_USERNAME_CHARS: usize = 256;
const MAX_PASSWORD_BYTES: usize = 1024;

fn too_long(username: &str, password: &SecretSlice<u8>) -> bool {
    username.chars().count() > MAX_USERNAME_CHARS
        || password.expose_secret().len() > MAX_PASSWORD_BYTES
}

impl DefaultAuthService {
    pub(crate) async fn register(
        &self,
        username: String,
        password: SecretSlice<u8>,
    ) -> Result<Tokens, AuthServiceError> {
        let username = username.trim().to_owned();
        if username.is_empty() || password.expose_secret().is_empty() {
            return Err(AuthServiceError::MissingCredentials);
        }
        if too_long(&username, &password) {
            return Err(AuthServiceError::CredentialsTooLong);
        }
        let password_hash = self.passwords.hash(password).await?;

        match self
            .users
            .insert_credentials(&username, password_hash)
            .await
        {
            Ok(user_id) => self
                .issue_tokens(user_id)
                .await
                .map_err(|e| AuthServiceError::SignInAfterRegister(user_id, Box::new(e))),
            Err(RepositoryError::UniqueViolation(_)) => {
                Err(AuthServiceError::UserAlreadyExists(username))
            }
            Err(e) => Err(e.into()),
        }
    }

    pub(crate) async fn login(
        &self,
        username: String,
        password: SecretSlice<u8>,
    ) -> Result<Tokens, AuthServiceError> {
        let username = username.trim().to_owned();
        if too_long(&username, &password) {
            return Err(AuthServiceError::LoginTooLong);
        }
        let Some(credentials) = self.users.get_credentials(&username).await? else {
            self.passwords.verify_dummy(password).await?;
            return Err(AuthServiceError::UserNotFound(username));
        };

        let password_matches = self
            .passwords
            .verify(password, credentials.password_hash)
            .await?;

        if !credentials.active {
            return Err(AuthServiceError::UserNotActive(credentials.user_id));
        }

        match password_matches {
            true => self.issue_tokens(credentials.user_id).await,
            false => Err(AuthServiceError::InvalidCredentials(username)),
        }
    }

    async fn issue_tokens(&self, user_id: UserId) -> Result<Tokens, AuthServiceError> {
        let access_token = self.issue_access_token(user_id).await?;
        let (refresh_token, refresh_token_expires_at) = self.issue_refresh_token(user_id).await?;

        Ok(Tokens {
            access_token,
            refresh_token,
            refresh_token_expires_at,
        })
    }

    pub(super) async fn issue_access_token(
        &self,
        user_id: UserId,
    ) -> Result<String, AuthServiceError> {
        let signed = self.signer.sign(user_id.0).await?;

        if let Some(key) = signed.new_key {
            let record = JwkRecord::try_from(key)?;
            self.jwk_cache.insert(&record).await?;
            self.local_jwks.insert(record);
        } else if !self.jwk_cache.contains(&signed.key_id).await? {
            let records = self.fetch_jwks().await?;
            self.jwk_cache.insert_many(&records).await?;
            self.local_jwks.replace(records);
        }

        Ok(signed.token)
    }
}
