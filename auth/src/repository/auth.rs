use std::fmt::Debug;

use sqlx::{PgPool, query_as, query_scalar};
use uuid::Uuid;

use crate::repository::errors::RepositoryError;

#[derive(Debug, sqlx::Type)]
#[sqlx(transparent)]
pub struct UserId(pub Uuid);

#[derive(Debug)]
pub struct Credentials {
    pub user_id: Uuid,
    pub password_hash: String,
    pub active: bool,
}

#[derive(Debug, sqlx::Type)]
#[sqlx(transparent)]
pub struct Active(pub bool);

#[tonic::async_trait]
pub trait AuthRepository: Debug + Send + Sync + 'static {
    async fn insert_credentials(
        &self,
        username: String,
        password_hash: String,
    ) -> Result<UserId, RepositoryError>;
    async fn get_credentials(
        &self,
        username: String,
    ) -> Result<Option<Credentials>, RepositoryError>;
    async fn verify(&self, user_id: Uuid) -> Result<Option<Active>, RepositoryError>;
}

#[derive(Debug)]
pub struct DbAuthRepository {
    db_pool: PgPool,
}

impl DbAuthRepository {
    pub fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }
}

#[tonic::async_trait]
impl AuthRepository for DbAuthRepository {
    async fn insert_credentials(
        &self,
        username: String,
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
        username: String,
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

    async fn verify(&self, user_id: Uuid) -> Result<Option<Active>, RepositoryError> {
        let is_active = query_scalar!(
            r#"SELECT active AS "active: Active" FROM public.auth WHERE user_id = $1"#,
            user_id
        )
        .fetch_optional(&self.db_pool)
        .await?;

        Ok(is_active)
    }
}
