use core::slice;

use async_trait::async_trait;
use redis::{AsyncCommands, Client, RedisResult, aio::ConnectionManager};
use uuid::Uuid;

use crate::verification_keys::{JwkRecord, VerificationKeyStore, VerificationKeyStoreError};

#[derive(Debug, Clone)]
pub struct RedisVerificationKeyStore {
    conn: ConnectionManager,
}

impl RedisVerificationKeyStore {
    pub async fn new(client: Client) -> RedisResult<Self> {
        let conn: ConnectionManager = client.get_connection_manager().await?;
        Ok(Self { conn })
    }

    fn redis_key(kid: &Uuid) -> String {
        format!("jwk:{}", kid)
    }
}

#[async_trait]
impl VerificationKeyStore for RedisVerificationKeyStore {
    async fn has_verification_key(&self, kid: &Uuid) -> Result<bool, VerificationKeyStoreError> {
        let mut con = self.conn.clone();
        Ok(con.exists(Self::redis_key(kid)).await?)
    }

    async fn push_verification_key(
        &self,
        key: &JwkRecord,
    ) -> Result<(), VerificationKeyStoreError> {
        self.push_multiple_verification_keys(slice::from_ref(key))
            .await
    }

    async fn push_multiple_verification_keys(
        &self,
        keys: &[JwkRecord],
    ) -> Result<(), VerificationKeyStoreError> {
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
