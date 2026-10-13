use redis::{ConnectionAddr, ConnectionInfo, IntoConnectionInfo, RedisConnectionInfo, RedisResult};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RedisSettings {
    pub host: String,
    pub port: u16,
    pub password: Option<SecretString>,
}

impl Default for RedisSettings {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 6379,
            password: None,
        }
    }
}

impl RedisSettings {
    pub fn connection_info(&self) -> RedisResult<ConnectionInfo> {
        let info = ConnectionAddr::Tcp(self.host.clone(), self.port).into_connection_info()?;
        Ok(match &self.password {
            Some(password) => info.set_redis_settings(
                RedisConnectionInfo::default().set_password(password.expose_secret()),
            ),
            None => info,
        })
    }
}
