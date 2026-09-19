use std::fmt::Debug;

use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

#[derive(Debug)]
struct RegisterFunctionReturn {
    user_id: Uuid,
}

#[derive(Debug)]
struct LoginFunctionReturn {
    user_id: Uuid,
    password_hash: String,
    active: bool,
}

#[derive(Debug)]
struct VerifyFunctionReturn {
    active: bool,
}

#[tonic::async_trait]
pub trait AuthRepository: Debug + Send + Sync + 'static {
    fn register(
        &self,
        username: String,
        password_hash: String,
        conn: PgConnection,
    ) -> anyhow::Result<String>;
    fn login(&self, username: String, password_hash: String) -> anyhow::Result<String>;
    fn verify(&self, user_id: Uuid) -> anyhow::Result<String>;
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

impl AuthRepository for DbAuthRepository {
    fn register(
        &self,
        _username: String,
        _password_hash: String,
        _conn: PgConnection,
    ) -> anyhow::Result<String> {
        let user_id = Uuid::new_v4();
        Ok(user_id.to_string())
    }

    fn login(&self, _username: String, _password_hash: String) -> anyhow::Result<String> {
        let user_id = Uuid::new_v4();
        Ok(user_id.to_string())
    }

    fn verify(&self, user_id: Uuid) -> anyhow::Result<String> {
        Ok(user_id.to_string())
    }
}
