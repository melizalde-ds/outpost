use redis::{
    ConnectionAddr, ConnectionInfo, IntoConnectionInfo, RedisConnectionInfo, RedisResult,
    aio::ConnectionManagerConfig,
};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RedisSettings {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    pub username: String,
    pub password: SecretString,
}

fn default_host() -> String {
    "localhost".to_string()
}

fn default_port() -> u16 {
    6379
}

impl RedisSettings {
    pub fn connection_info(&self) -> RedisResult<ConnectionInfo> {
        let info = ConnectionAddr::Tcp(self.host.clone(), self.port).into_connection_info()?;
        Ok(info.set_redis_settings(
            RedisConnectionInfo::default()
                .set_username(&self.username)
                .set_password(self.password.expose_secret()),
        ))
    }

    pub fn connection_manager_config(&self) -> ConnectionManagerConfig {
        ConnectionManagerConfig::new().set_number_of_retries(0)
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    use super::*;

    #[test]
    fn connection_info_logs_in_as_the_configured_user() {
        // Arrange
        let settings: RedisSettings =
            serde_json::from_str(r#"{ "username": "auth", "password": "secret" }"#).unwrap();

        // Act
        let info = settings.connection_info().unwrap();

        // Assert
        assert_eq!(
            (
                info.redis_settings().username(),
                info.redis_settings().password()
            ),
            (Some("auth"), Some("secret"))
        );
    }

    #[rstest]
    #[case::missing_username(r#"{ "password": "secret" }"#)]
    #[case::missing_password(r#"{ "username": "auth" }"#)]
    fn deserialize_missing_credentials_is_rejected(#[case] json: &str) {
        // Act
        let result = serde_json::from_str::<RedisSettings>(json);

        // Assert
        assert!(result.is_err());
    }
}
