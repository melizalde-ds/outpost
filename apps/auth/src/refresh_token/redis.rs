use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use redis::{AsyncCommands, Client, RedisResult, aio::ConnectionManager};
use uuid::Uuid;

use crate::refresh_token::{RefreshToken, RefreshTokenStore, RefreshTokenStoreError};

#[derive(Debug, Clone)]
pub struct RedisRefreshTokenStore {
    conn: ConnectionManager,
}

impl RedisRefreshTokenStore {
    pub async fn new(client: Client) -> RedisResult<Self> {
        let conn: ConnectionManager = client.get_connection_manager().await?;
        Ok(Self { conn })
    }

    fn redis_key(hash: &[u8; 32]) -> String {
        format!("refresh_token:{}", URL_SAFE_NO_PAD.encode(hash))
    }
}

#[async_trait]
impl RefreshTokenStore for RedisRefreshTokenStore {
    async fn insert(&self, token: &RefreshToken) -> Result<(), RefreshTokenStoreError> {
        let mut con = self.conn.clone();
        let redis_key = Self::redis_key(&token.hash);
        let mut pipe = redis::pipe();

        pipe.atomic()
            .set(&redis_key, token.user_id)
            .ignore()
            .pexpire_at(&redis_key, token.exp.timestamp_millis())
            .ignore();
        pipe.query_async::<()>(&mut con).await?;
        Ok(())
    }

    async fn find(&self, hash: &[u8; 32]) -> Result<Option<Uuid>, RefreshTokenStoreError> {
        let mut con = self.conn.clone();
        Ok(con.get(Self::redis_key(hash)).await?)
    }

    async fn revoke(&self, hash: &[u8; 32]) -> Result<(), RefreshTokenStoreError> {
        let mut con = self.conn.clone();
        con.del::<_, ()>(Self::redis_key(hash)).await?;
        Ok(())
    }
}
