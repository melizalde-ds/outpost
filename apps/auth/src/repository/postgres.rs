use async_trait::async_trait;
use secrecy::ExposeSecret;
use sqlx::{PgPool, query_as, query_scalar};
use uuid::Uuid;

use crate::{
    repository::{AuthRepository, Credentials, UserId, errors::RepositoryError},
    settings::get_settings,
};

#[derive(Debug)]
pub struct PostgresAuthRepository {
    db_pool: PgPool,
}

impl PostgresAuthRepository {
    pub async fn new() -> anyhow::Result<Self> {
        let settings = get_settings();
        let db_pool =
            sqlx::PgPool::connect(settings.database.connection_string().expose_secret()).await?;
        Ok(Self { db_pool })
    }
}

#[async_trait]
impl AuthRepository for PostgresAuthRepository {
    async fn insert_credentials(
        &self,
        username: &str,
        password_hash: String,
    ) -> Result<UserId, RepositoryError> {
        let id: UserId = query_scalar!(
            r#"SELECT public.register(p_username => $1, p_password_hash => $2) AS "id!: UserId""#,
            username,
            password_hash
        )
        .fetch_one(&self.db_pool)
        .await?;

        Ok(id)
    }

    async fn get_credentials(
        &self,
        username: &str,
    ) -> Result<Option<Credentials>, RepositoryError> {
        let query_result = query_as!(
            Credentials,
            r#"SELECT user_id AS "user_id!", password_hash AS "password_hash!", active AS "active!" FROM public.get_credentials(p_username => $1)"#,
            username
        )
        .fetch_optional(&self.db_pool)
        .await?;

        Ok(query_result)
    }

    async fn is_active(&self, user_id: &Uuid) -> Result<bool, RepositoryError> {
        let is_active = query_scalar!(
            r#"SELECT public.is_active(p_user_id => $1) AS "active!""#,
            user_id
        )
        .fetch_one(&self.db_pool)
        .await?;

        Ok(is_active)
    }
}
