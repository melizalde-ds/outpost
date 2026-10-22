use std::time::Duration;

use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

#[derive(Debug, Deserialize)]
pub struct DatabaseSettings {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    pub username: String,
    pub password: SecretString,
    #[serde(default = "default_database_name")]
    pub database_name: String,
    #[serde(default)]
    pub pool: PoolSettings,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct PoolSettings {
    pub max_connections: u32,
    pub min_connections: u32,
    pub idle_timeout_secs: u64,
}

fn default_host() -> String {
    "localhost".to_string()
}

fn default_port() -> u16 {
    5432
}

fn default_database_name() -> String {
    "auth".to_string()
}

impl Default for PoolSettings {
    fn default() -> Self {
        Self {
            max_connections: 5,
            min_connections: 0,
            idle_timeout_secs: 60,
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
            .idle_timeout(Duration::from_secs(self.pool.idle_timeout_secs))
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    use super::*;

    #[test]
    fn pool_options_use_the_pool_settings() {
        // Arrange
        let settings: DatabaseSettings = serde_json::from_str(
            r#"{
                "username": "auth_app",
                "password": "secret",
                "pool": { "max_connections": 7, "min_connections": 2, "idle_timeout_secs": 30 }
            }"#,
        )
        .unwrap();

        // Act
        let options = settings.pool_options();

        // Assert
        assert_eq!(
            (
                options.get_max_connections(),
                options.get_min_connections(),
                options.get_idle_timeout(),
            ),
            (7, 2, Some(Duration::from_secs(30)))
        );
    }

    #[rstest]
    #[case::missing_username(r#"{ "password": "secret" }"#)]
    #[case::missing_password(r#"{ "username": "auth_app" }"#)]
    fn deserialize_missing_credentials_is_rejected(#[case] json: &str) {
        // Act
        let result = serde_json::from_str::<DatabaseSettings>(json);

        // Assert
        assert!(result.is_err());
    }
}
