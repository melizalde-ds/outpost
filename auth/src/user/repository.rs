use std::fmt::Debug;

use sqlx::{PgConnection, PgPool, query_as};
use uuid::Uuid;

use crate::user::errors::RepositoryError;

#[derive(Debug, sqlx::Type)]
#[sqlx(transparent)]
struct UserId(pub Uuid);

#[derive(Debug)]
struct Credentials {
    pub user_id: Uuid,
    pub password_hash: String,
    pub active: bool,
}

#[derive(Debug, sqlx::Type)]
#[sqlx(transparent)]
struct Active(pub bool);

#[tonic::async_trait]
pub trait AuthRepository: Debug + Send + Sync + 'static {
    async fn register(
        &self,
        username: String,
        password_hash: String,
        conn: PgConnection,
    ) -> anyhow::Result<Uuid, RepositoryError>;
    async fn login(
        &self,
        username: String,
        password_hash: String,
    ) -> anyhow::Result<Option<Credentials>, RepositoryError>;
    async fn verify(&self, user_id: Uuid) -> anyhow::Result<String, RepositoryError>;
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
    async fn register(
        &self,
        _username: String,
        _password_hash: String,
        _conn: PgConnection,
    ) -> anyhow::Result<Uuid, RepositoryError> {
        let user_id = Uuid::new_v4();

        Ok(user_id)
    }

    async fn login(
        &self,
        username: String,
        _password_hash: String,
    ) -> anyhow::Result<Option<Credentials>, RepositoryError> {
        let query_result = query_as!(
            Credentials,
            r#"SELECT user_id AS "user_id!", password_hash AS "password_hash!", active AS "active!" FROM public.get_credentials(p_username => $1)"#,
            username
        )
        .fetch_optional(&self.db_pool)
        .await?;

        Ok(query_result)
    }

    async fn verify(&self, user_id: Uuid) -> anyhow::Result<String, RepositoryError> {
        Ok(user_id.to_string())
    }
}
