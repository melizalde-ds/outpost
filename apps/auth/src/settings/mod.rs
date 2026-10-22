use config::Config;
use serde::Deserialize;

mod database;
mod grpc;
mod http;
mod password;
mod redis;
mod refresh_token;

pub use database::DatabaseSettings;
pub use grpc::{GrpcEndpoint, GrpcSettings};
pub use http::HttpSettings;
pub use password::PasswordSettings;
pub use redis::RedisSettings;
pub use refresh_token::RefreshTokenSettings;

#[derive(Debug, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub database: DatabaseSettings,
    pub password: PasswordSettings,
    pub grpc: GrpcSettings,
    pub http: HttpSettings,
    #[serde(default)]
    pub redis: RedisSettings,
    #[serde(default)]
    pub refresh_token: RefreshTokenSettings,
}

impl Settings {
    pub fn load() -> anyhow::Result<Self> {
        Ok(Config::builder()
            .add_source(
                config::Environment::default()
                    .separator("__")
                    .prefix("AUTH"),
            )
            .build()?
            .try_deserialize()?)
    }
}
