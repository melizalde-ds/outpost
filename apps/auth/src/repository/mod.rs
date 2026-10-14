use std::fmt::Debug;

use async_trait::async_trait;

use uuid::Uuid;

mod errors;
mod token_family;
mod user;

pub use errors::RepositoryError;
pub use token_family::PostgresTokenFamilyRepository;
pub use user::PostgresUserRepository;

#[derive(Debug, sqlx::Type)]
#[sqlx(transparent)]
pub struct UserId(pub Uuid);

#[derive(Debug, Clone, Copy, sqlx::Type)]
#[sqlx(transparent)]
pub struct TokenFamilyId(pub Uuid);

#[derive(Debug, Clone, Copy)]
pub enum RevocationReason {
    Reuse,
    Logout,
    LogoutAll,
    Inactive,
    Blocked,
}

impl RevocationReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            RevocationReason::Reuse => "reuse",
            RevocationReason::Logout => "logout",
            RevocationReason::LogoutAll => "logout_all",
            RevocationReason::Inactive => "inactive",
            RevocationReason::Blocked => "blocked",
        }
    }
}

#[derive(Debug)]
pub struct Credentials {
    pub user_id: Uuid,
    pub password_hash: String,
    pub active: bool,
}

#[async_trait]
pub trait UserRepository: Debug + Send + Sync + 'static {
    async fn insert_credentials(
        &self,
        username: &str,
        password_hash: String,
    ) -> Result<UserId, RepositoryError>;
    async fn get_credentials(&self, username: &str)
    -> Result<Option<Credentials>, RepositoryError>;
    async fn is_active(&self, user_id: &Uuid) -> Result<bool, RepositoryError>;
}

#[async_trait]
pub trait TokenFamilyRepository: Debug + Send + Sync + 'static {
    async fn insert(&self, user_id: &UserId) -> Result<TokenFamilyId, RepositoryError>;
    async fn find_unrevoked_by_user_id(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<TokenFamilyId>, RepositoryError>;
    async fn revoke(
        &self,
        family_id: &TokenFamilyId,
        reason: RevocationReason,
    ) -> Result<bool, RepositoryError>;
    async fn revoke_all(
        &self,
        user_id: &UserId,
        reason: RevocationReason,
    ) -> Result<Vec<TokenFamilyId>, RepositoryError>;
}
