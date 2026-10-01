use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use std::time::Duration;

#[derive(Debug, Deserialize)]
pub struct DatabaseSettings {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: SecretString,
    pub database_name: String,
    pub pool: PoolSettings,
}

#[derive(Debug, Deserialize)]
pub struct PoolSettings {
    pub max_connections: u32,
    pub min_connections: u32,
    pub idle_timeout: Option<Duration>,
}

impl Default for DatabaseSettings {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 5432,
            username: "postgres".to_string(),
            password: "password".into(),
            database_name: "auth".to_string(),
            pool: PoolSettings {
                max_connections: 5,
                min_connections: 0,
                idle_timeout: Some(Duration::from_secs(60)),
            },
        }
    }
}

impl DatabaseSettings {
    pub fn connection_options(&self) -> PgConnectOptions {
        PgConnectOptions::new()
            .host(&self.host)
            .port(self.port)
            .username(&self.username)
            .password(self.password.expose_secret())
            .database(&self.database_name)
    }

    pub fn pool_options(&self) -> PgPoolOptions {
        PgPoolOptions::new()
            .max_connections(self.pool.max_connections)
            .min_connections(self.pool.min_connections)
            .idle_timeout(self.pool.idle_timeout)
    }
}
