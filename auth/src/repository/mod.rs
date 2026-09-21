use std::fmt::Debug;

use async_trait::async_trait;

use uuid::Uuid;

use crate::repository::errors::RepositoryError;

pub(crate) mod errors;
pub(crate) mod postgres;

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
