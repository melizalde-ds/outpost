use config::Config;
use serde::Deserialize;

use crate::settings::{
    database::DatabaseSettings, grpc::GrpcSettings, password::PasswordSettings,
    server::ServerSettings,
};

pub(crate) mod database;
pub(crate) mod grpc;
pub(crate) mod password;
pub(crate) mod server;

#[derive(Debug, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub database: DatabaseSettings,
    pub password: PasswordSettings,
    #[serde(default)]
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
