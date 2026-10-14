use async_trait::async_trait;
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
    async fn insert(&self, user_id: &UserId) -> Result<TokenFamilyId, RepositoryError> {
        let family_id: TokenFamilyId = query_scalar!(
            r#"SELECT public.create_token_family(p_user_id => $1) AS "id!: TokenFamilyId""#,
            user_id.0
        )
        .fetch_one(&self.db_pool)
        .await?;

        Ok(family_id)
    }

    async fn find_unrevoked_by_user_id(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<TokenFamilyId>, RepositoryError> {
        let family_ids: Vec<TokenFamilyId> = query_scalar!(
            r#"SELECT family_id AS "id!: TokenFamilyId" FROM public.get_user_token_families(p_user_id => $1)"#,
            user_id.0
        )
        .fetch_all(&self.db_pool)
        .await?;

        Ok(family_ids)
    }

    async fn revoke(
        &self,
        family_id: &TokenFamilyId,
        reason: RevocationReason,
    ) -> Result<bool, RepositoryError> {
        let revoked = query_scalar!(
            r#"SELECT public.revoke_token_family(p_family_id => $1, p_reason => $2) AS "revoked!""#,
            family_id.0,
            reason.as_str()
        )
        .fetch_one(&self.db_pool)
        .await?;

        Ok(revoked == 1)
    }

    async fn revoke_all(
        &self,
        user_id: &UserId,
        reason: RevocationReason,
    ) -> Result<Vec<TokenFamilyId>, RepositoryError> {
        let family_ids: Vec<TokenFamilyId> = query_scalar!(
            r#"SELECT family_id AS "id!: TokenFamilyId" FROM public.revoke_user_token_families(p_user_id => $1, p_reason => $2)"#,
            user_id.0,
            reason.as_str()
        )
        .fetch_all(&self.db_pool)
        .await?;

        Ok(family_ids)
    }
}
