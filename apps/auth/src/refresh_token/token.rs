use std::sync::Arc;

use async_trait::async_trait;
use chrono::{Duration, Utc};

use crate::refresh_token::redis::{Entry, RefreshTokenStore};
use crate::refresh_token::{
    IssuedToken, RefreshToken, RefreshTokenError, RefreshTokenService, Session,
};
use crate::repository::{RevocationReason, TokenFamilyId, TokenFamilyRepository, UserId};
use crate::settings::RefreshTokenSettings;

#[derive(Debug)]
pub struct OutpostRefreshTokenService {
    store: Arc<dyn RefreshTokenStore>,
    families: Arc<dyn TokenFamilyRepository>,
    idle_timeout: Duration,
    max_age: Duration,
}

impl OutpostRefreshTokenService {
    pub fn new(
        store: Arc<dyn RefreshTokenStore>,
        families: Arc<dyn TokenFamilyRepository>,
        settings: &RefreshTokenSettings,
    ) -> Self {
        Self {
            store,
            families,
            idle_timeout: Duration::seconds(settings.idle_timeout_secs as i64),
            max_age: Duration::seconds(settings.max_age_secs as i64),
        }
    }

    async fn reused(&self, family_id: TokenFamilyId) -> RefreshTokenError {
        match self
            .families
            .revoke(&family_id, RevocationReason::Reuse)
            .await
        {
            Ok(_) => RefreshTokenError::Reused(family_id),
            Err(e) => e.into(),
        }
    }
}

#[async_trait]
impl RefreshTokenService for OutpostRefreshTokenService {
    async fn issue(&self, user_id: UserId) -> Result<IssuedToken, RefreshTokenError> {
        let now = Utc::now();
        let family_exp = now + self.max_age;
        let exp = (now + self.idle_timeout).min(family_exp);

        let family_id = self.families.insert(&user_id).await?;
        let (token, refresh_token) = RefreshToken::generate(user_id, family_id, exp, family_exp)?;
        self.store.insert(&refresh_token).await?;

        Ok(IssuedToken { token, exp })
    }

    async fn validate(&self, token: &str) -> Result<Session, RefreshTokenError> {
        let (family_id, hash) = RefreshToken::parse(token).ok_or(RefreshTokenError::Invalid)?;

        let (user_id, family_exp) = match self.store.find(&hash).await? {
            None => return Err(RefreshTokenError::Invalid),
            Some(Entry::Rotated) => return Err(self.reused(family_id).await),
            Some(Entry::Live {
                user_id,
                family_exp,
            }) => (UserId(user_id), family_exp),
        };

        let families = self.families.find_unrevoked_by_user_id(&user_id).await?;
        if !families.contains(&family_id) {
            return Err(RefreshTokenError::Invalid);
        }

        Ok(Session {
            user_id,
            family_id,
            family_exp,
            hash,
        })
    }

    async fn rotate(&self, session: &Session) -> Result<IssuedToken, RefreshTokenError> {
        let exp = (Utc::now() + self.idle_timeout).min(session.family_exp);
        let (token, refresh_token) =
            RefreshToken::generate(session.user_id, session.family_id, exp, session.family_exp)?;

        match self.store.rotate(&session.hash, &refresh_token).await? {
            Some(Entry::Live { .. }) => Ok(IssuedToken { token, exp }),
            Some(Entry::Rotated) => Err(self.reused(session.family_id).await),
            None => Err(RefreshTokenError::Invalid),
        }
    }

    async fn revoke(
        &self,
        session: &Session,
        reason: RevocationReason,
    ) -> Result<(), RefreshTokenError> {
        self.store.revoke(&session.hash).await?;
        self.families.revoke(&session.family_id, reason).await?;
        Ok(())
    }
}
