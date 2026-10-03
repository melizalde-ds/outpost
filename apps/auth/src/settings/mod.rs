use config::Config;
use serde::Deserialize;

mod database;
mod grpc;
mod password;
mod server;

pub use database::DatabaseSettings;
pub use grpc::{GrpcEndpoint, GrpcSettings};
pub use password::PasswordSettings;
pub use server::ServerSettings;

#[derive(Debug, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub database: DatabaseSettings,
    pub password: PasswordSettings,
    pub grpc: GrpcSettings,
    #[serde(default)]
    pub server: ServerSettings,
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
