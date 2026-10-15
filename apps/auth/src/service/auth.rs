use secrecy::SecretSlice;

use crate::{
    jwk_cache::JwkRecord,
    repository::{RepositoryError, UserId},
    service::{AuthServiceError, DefaultAuthService, Tokens},
};

impl DefaultAuthService {
    pub(crate) async fn register(
        &self,
        username: String,
        password: SecretSlice<u8>,
    ) -> Result<Tokens, AuthServiceError> {
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
        } else if !self.jwk_cache.contains(&signed.key_id).await? {
            let records = self
                .signer
                .get_verification_keys()
                .await?
                .into_iter()
                .map(JwkRecord::try_from)
                .collect::<Result<Vec<_>, _>>()?;
            self.jwk_cache.insert_many(&records).await?;
        }

        Ok(signed.token)
    }
}
