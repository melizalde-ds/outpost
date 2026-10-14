use std::fmt::Debug;

use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use redis::{AsyncCommands, Client, RedisResult, aio::ConnectionManager};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::refresh_token::{RefreshToken, RefreshTokenError};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Entry {
    Live {
        user_id: Uuid,
        family_exp: DateTime<Utc>,
    },
    Rotated,
}

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

    fn decode(value: Option<String>) -> Result<Option<Entry>, RefreshTokenError> {
        Ok(value.map(|v| serde_json::from_str(&v)).transpose()?)
    }
}

#[async_trait]
pub trait RefreshTokenStore: Debug + Send + Sync + 'static {
    async fn insert(&self, token: &RefreshToken) -> Result<(), RefreshTokenError>;
    async fn find(&self, hash: &[u8; 32]) -> Result<Option<Entry>, RefreshTokenError>;
    async fn rotate(
        &self,
        old: &[u8; 32],
        new: &RefreshToken,
    ) -> Result<Option<Entry>, RefreshTokenError>;
    async fn revoke(&self, hash: &[u8; 32]) -> Result<(), RefreshTokenError>;
}

#[async_trait]
impl RefreshTokenStore for RedisRefreshTokenStore {
    async fn insert(&self, token: &RefreshToken) -> Result<(), RefreshTokenError> {
        let mut con = self.conn.clone();
        let redis_key = Self::redis_key(&token.hash);
        let entry = serde_json::to_string(&Entry::Live {
            user_id: token.user_id.0,
            family_exp: token.family_exp,
        })?;
        let mut pipe = redis::pipe();

        pipe.atomic()
            .set(&redis_key, entry)
            .ignore()
            .pexpire_at(&redis_key, token.exp.timestamp_millis())
            .ignore();
        pipe.query_async::<()>(&mut con).await?;
        Ok(())
    }

    async fn find(&self, hash: &[u8; 32]) -> Result<Option<Entry>, RefreshTokenError> {
        let mut con = self.conn.clone();
        Self::decode(con.get(Self::redis_key(hash)).await?)
    }

    async fn rotate(
        &self,
        old: &[u8; 32],
        new: &RefreshToken,
    ) -> Result<Option<Entry>, RefreshTokenError> {
        let mut con = self.conn.clone();
        let rotated = serde_json::to_string(&Entry::Rotated)?;
        let previous: Option<String> = redis::cmd("SET")
            .arg(Self::redis_key(old))
            .arg(rotated)
            .arg("XX")
            .arg("GET")
            .arg("KEEPTTL")
            .query_async(&mut con)
            .await?;
        let previous = Self::decode(previous)?;

        if let Some(Entry::Live { .. }) = previous {
            self.insert(new).await?;
        }
        Ok(previous)
    }

    async fn revoke(&self, hash: &[u8; 32]) -> Result<(), RefreshTokenError> {
        let mut con = self.conn.clone();
        con.del::<_, ()>(Self::redis_key(hash)).await?;
        Ok(())
    }
}
