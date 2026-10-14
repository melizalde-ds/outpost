use chrono::{DateTime, Utc};

use crate::{
    refresh_token::{Entry, RefreshToken},
    repository::{RevocationReason, TokenFamilyId, UserId},
    service::{AuthServiceError, DefaultAuthService, Tokens},
};

struct Session {
    user_id: UserId,
    family_id: TokenFamilyId,
    family_exp: DateTime<Utc>,
    hash: [u8; 32],
}

impl DefaultAuthService {
    pub(crate) async fn refresh(&self, token: &str) -> Result<Tokens, AuthServiceError> {
        let session = self.validate_refresh_token(token).await?;

        if !self.users.is_active(&session.user_id).await? {
            self.revoke_refresh_token(&session, RevocationReason::Inactive)
                .await?;
            return Err(AuthServiceError::UserNotActive(session.user_id));
        }

        let access_token = self.issue_access_token(session.user_id).await?;
        let (refresh_token, refresh_token_expires_at) = self.rotate_refresh_token(&session).await?;

        Ok(Tokens {
            access_token,
            refresh_token,
            refresh_token_expires_at,
        })
    }

    pub(super) async fn issue_refresh_token(
        &self,
        user_id: UserId,
    ) -> Result<(String, DateTime<Utc>), AuthServiceError> {
        let now = Utc::now();
        let family_exp = now + self.max_age;
        let exp = (now + self.idle_timeout).min(family_exp);

        let family_id = self.families.insert(&user_id).await?;
        let (token, refresh_token) = RefreshToken::generate(user_id, family_id, exp, family_exp)?;
        self.refresh_tokens.insert(&refresh_token).await?;

        Ok((token, exp))
    }

    async fn validate_refresh_token(&self, token: &str) -> Result<Session, AuthServiceError> {
        let (family_id, hash) =
            RefreshToken::parse(token).ok_or(AuthServiceError::InvalidRefreshToken)?;

        let (user_id, family_exp) = match self.refresh_tokens.find(&hash).await? {
            None => return Err(AuthServiceError::InvalidRefreshToken),
            Some(Entry::Rotated) => return Err(self.reused(family_id).await),
            Some(Entry::Live {
                user_id,
                family_exp,
            }) => (user_id, family_exp),
        };

        let families = self.families.find_unrevoked_by_user_id(&user_id).await?;
        if !families.contains(&family_id) {
            return Err(AuthServiceError::InvalidRefreshToken);
        }

        Ok(Session {
            user_id,
            family_id,
            family_exp,
            hash,
        })
    }

    async fn rotate_refresh_token(
        &self,
        session: &Session,
    ) -> Result<(String, DateTime<Utc>), AuthServiceError> {
        let exp = (Utc::now() + self.idle_timeout).min(session.family_exp);
        let (token, refresh_token) =
            RefreshToken::generate(session.user_id, session.family_id, exp, session.family_exp)?;

        match self
            .refresh_tokens
            .rotate(&session.hash, &refresh_token)
            .await?
        {
            Some(Entry::Live { .. }) => Ok((token, exp)),
            Some(Entry::Rotated) => Err(self.reused(session.family_id).await),
            None => Err(AuthServiceError::InvalidRefreshToken),
        }
    }

    async fn revoke_refresh_token(
        &self,
        session: &Session,
        reason: RevocationReason,
    ) -> Result<(), AuthServiceError> {
        self.refresh_tokens.revoke(&session.hash).await?;
        self.families.revoke(&session.family_id, reason).await?;
        Ok(())
    }

    async fn reused(&self, family_id: TokenFamilyId) -> AuthServiceError {
        match self
            .families
            .revoke(&family_id, RevocationReason::Reuse)
            .await
        {
            Ok(_) => AuthServiceError::RefreshTokenReused(family_id),
            Err(e) => e.into(),
        }
    }
}
