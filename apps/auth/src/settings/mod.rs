use std::sync::OnceLock;

use config::Config;
use serde::Deserialize;

use crate::settings::{
    auth::AuthSettings, database::DatabaseSettings, grpc::GrpcSettings, server::ServerSettings,
};

pub(crate) mod auth;
pub(crate) mod database;
pub(crate) mod grpc;
pub(crate) mod server;

static SETTINGS: OnceLock<Settings> = OnceLock::new();

pub fn init() -> anyhow::Result<&'static Settings> {
    let settings = Settings::load()?;
    SETTINGS
        .set(settings)
        .map_err(|_| anyhow::anyhow!("Settings already initialized"))?;
    Ok(SETTINGS.get().expect("settings were just initialized"))
}

pub fn get_settings() -> &'static Settings {
    SETTINGS
        .get()
        .expect("Settings not initialized. Call init() first.")
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub database: DatabaseSettings,
    #[serde(default)]
    pub auth: AuthSettings,
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
