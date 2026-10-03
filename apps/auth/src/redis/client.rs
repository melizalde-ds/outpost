use core::slice;

use redis::{Client, RedisResult, aio::ConnectionManager};
use uuid::Uuid;

use crate::cache::JwkRecord;
#[derive(Debug, Clone)]
pub struct RedisClient {
    conn: ConnectionManager,
}

impl RedisClient {
    pub async fn new(client: Client) -> RedisResult<Self> {
        let conn: ConnectionManager = client.get_connection_manager().await?;
        Ok(Self { conn })
    }

    pub(crate) async fn push_verification_key(&self, key: &JwkRecord) -> RedisResult<()> {
        self.push_multiple_verification_keys(slice::from_ref(key))
            .await
    }

    pub(crate) async fn push_multiple_verification_keys(
        &self,
        keys: &[JwkRecord],
    ) -> RedisResult<()> {
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
        pipe.query_async(&mut con).await?
    }

    fn redis_key(kid: &Uuid) -> String {
        format!("jwk:{}", kid)
    }
}
