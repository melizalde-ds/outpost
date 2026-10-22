use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, query_scalar};

use crate::repository::{
    RepositoryError, RevocationReason, TokenFamilyId, TokenFamilyRepository, UserId,
};

#[derive(Debug)]
pub struct PostgresTokenFamilyRepository {
    db_pool: PgPool,
}

impl PostgresTokenFamilyRepository {
    pub fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }
}

#[async_trait]
impl TokenFamilyRepository for PostgresTokenFamilyRepository {
    async fn insert(
        &self,
        user_id: &UserId,
        expires_at: DateTime<Utc>,
    ) -> Result<TokenFamilyId, RepositoryError> {
        let family_id: TokenFamilyId = query_scalar!(
            r#"SELECT public.create_token_family(p_user_id => $1, p_expires_at => $2) AS "id!: TokenFamilyId""#,
            user_id.0,
            expires_at
        )
        .fetch_one(&self.db_pool)
        .await?;

        Ok(family_id)
    }

    async fn is_live(
        &self,
        family_id: &TokenFamilyId,
        user_id: &UserId,
    ) -> Result<bool, RepositoryError> {
        let live = query_scalar!(
            r#"SELECT public.is_token_family_live(p_family_id => $1, p_user_id => $2) AS "live!""#,
            family_id.0,
            user_id.0
        )
        .fetch_one(&self.db_pool)
        .await?;

        Ok(live)
    }

    async fn revoke(
        &self,
        family_id: &TokenFamilyId,
        reason: RevocationReason,
    ) -> Result<(), RepositoryError> {
        query_scalar!(
            r#"SELECT public.revoke_token_family(p_family_id => $1, p_reason => $2) AS "revoked!""#,
            family_id.0,
            reason.as_str()
        )
        .fetch_one(&self.db_pool)
        .await?;

        Ok(())
    }

    async fn revoke_all(
        &self,
        user_id: &UserId,
        reason: RevocationReason,
    ) -> Result<(), RepositoryError> {
        query_scalar!(
            r#"SELECT family_id AS "id!: TokenFamilyId" FROM public.revoke_user_token_families(p_user_id => $1, p_reason => $2)"#,
            user_id.0,
            reason.as_str()
        )
        .fetch_all(&self.db_pool)
        .await?;

        Ok(())
    }
}
