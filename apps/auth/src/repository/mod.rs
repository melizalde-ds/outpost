use std::fmt::Debug;

use async_trait::async_trait;

use uuid::Uuid;

mod errors;
mod postgres;

pub use errors::RepositoryError;
pub use postgres::PostgresAuthRepository;

#[derive(Debug, sqlx::Type)]
#[sqlx(transparent)]
pub struct UserId(pub Uuid);

#[derive(Debug)]
pub struct Credentials {
    pub user_id: Uuid,
    pub password_hash: String,
    pub active: bool,
}

#[async_trait]
pub trait AuthRepository: Debug + Send + Sync + 'static {
    async fn insert_credentials(
        &self,
        username: &str,
        password_hash: String,
    ) -> Result<UserId, RepositoryError>;
    async fn get_credentials(&self, username: &str)
    -> Result<Option<Credentials>, RepositoryError>;
    async fn is_active(&self, user_id: &Uuid) -> Result<bool, RepositoryError>;
}
