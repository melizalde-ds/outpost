use async_trait::async_trait;
use chrono::{DateTime, Utc};
use redis::{AsyncCommands, Client, RedisResult, aio::ConnectionManager};
use uuid::Uuid;

use crate::refresh_token::{RefreshTokenStore, RefreshTokenStoreError};

#[derive(Debug, Clone)]
pub struct RedisRefreshTokenStore {
    conn: ConnectionManager,
}

impl RedisRefreshTokenStore {
    pub async fn new(client: Client) -> RedisResult<Self> {
        let conn: ConnectionManager = client.get_connection_manager().await?;
        Ok(Self { conn })
    }

    fn redis_key(id: &Uuid) -> String {
        format!("refresh_token:{}", id)
    }
}

#[async_trait]
impl RefreshTokenStore for RedisRefreshTokenStore {
    async fn insert(
        &self,
        id: &Uuid,
        user_id: &Uuid,
        exp: DateTime<Utc>,
    ) -> Result<(), RefreshTokenStoreError> {
        let mut con = self.conn.clone();
        let redis_key = Self::redis_key(id);
        let mut pipe = redis::pipe();

        pipe.atomic()
            .set(&redis_key, user_id)
            .ignore()
            .pexpire_at(&redis_key, exp.timestamp_millis())
            .ignore();
        pipe.query_async::<()>(&mut con).await?;
        Ok(())
    }

    async fn find(&self, id: &Uuid) -> Result<Option<Uuid>, RefreshTokenStoreError> {
        let mut con = self.conn.clone();
        Ok(con.get(Self::redis_key(id)).await?)
    }

    async fn revoke(&self, id: &Uuid) -> Result<(), RefreshTokenStoreError> {
        let mut con = self.conn.clone();
        con.del::<_, ()>(Self::redis_key(id)).await?;
        Ok(())
    }
}
