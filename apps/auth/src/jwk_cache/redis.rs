use core::slice;

use async_trait::async_trait;
use redis::{AsyncCommands, Client, RedisResult, aio::ConnectionManager};
use uuid::Uuid;

use crate::jwk_cache::{JwkCache, JwkCacheError, JwkRecord};

#[derive(Debug, Clone)]
pub struct RedisJwkCache {
    conn: ConnectionManager,
}

impl RedisJwkCache {
    pub async fn new(client: Client) -> RedisResult<Self> {
        let conn: ConnectionManager = client.get_connection_manager().await?;
        Ok(Self { conn })
    }

    fn redis_key(kid: &Uuid) -> String {
        format!("jwk:{}", kid)
    }
}

#[async_trait]
impl JwkCache for RedisJwkCache {
    async fn contains(&self, kid: &Uuid) -> Result<bool, JwkCacheError> {
        let mut con = self.conn.clone();
        Ok(con.exists(Self::redis_key(kid)).await?)
    }

    async fn insert(&self, key: &JwkRecord) -> Result<(), JwkCacheError> {
        self.insert_many(slice::from_ref(key)).await
    }

    async fn insert_many(&self, keys: &[JwkRecord]) -> Result<(), JwkCacheError> {
        let mut con = self.conn.clone();
        let mut pipe = redis::pipe();

        pipe.atomic();
        for key in keys {
            let redis_key = Self::redis_key(&key.kid);
            pipe.cmd("JSON.SET")
                .arg(&redis_key)
                .arg("$")
                .arg(key.json.get())
                .ignore()
                .pexpire_at(&redis_key, key.exp.timestamp_millis())
                .ignore();
        }
        pipe.query_async::<()>(&mut con).await?;
        Ok(())
    }
}
