use std::{num::NonZeroU64, time::Duration};

use serde::Deserialize;

use super::grpc::host_port;

#[derive(Debug, Deserialize)]
pub struct HttpSettings {
    pub host: String,
    pub port: u16,
    // Non-zero: a zero interval panics in tokio, and a zero timeout fails every request.
    #[serde(default = "default_jwks_refresh_secs")]
    pub jwks_refresh_secs: NonZeroU64,
    #[serde(default = "default_request_timeout_secs")]
    pub request_timeout_secs: NonZeroU64,
}

fn default_jwks_refresh_secs() -> NonZeroU64 {
    NonZeroU64::new(60).unwrap()
}

pub(super) fn default_request_timeout_secs() -> NonZeroU64 {
    NonZeroU64::new(10).unwrap()
}

impl HttpSettings {
    pub fn address(&self) -> String {
        host_port(&self.host, self.port)
    }

    pub fn jwks_refresh(&self) -> Duration {
        Duration::from_secs(self.jwks_refresh_secs.get())
    }

    pub fn request_timeout(&self) -> Duration {
        Duration::from_secs(self.request_timeout_secs.get())
    }
}

#[cfg(test)]
mod tests {
    use config::{Config, ConfigBuilder, ConfigError, builder::DefaultState};
    use rstest::rstest;

    use super::*;

    fn required() -> ConfigBuilder<DefaultState> {
        Config::builder()
            .set_override("host", "::1")
            .unwrap()
            .set_override("port", "8080")
            .unwrap()
    }

    fn load(builder: ConfigBuilder<DefaultState>) -> Result<HttpSettings, ConfigError> {
        builder.build()?.try_deserialize()
    }

    #[test]
    fn deserialize_nothing_set_reloads_keys_every_60s_and_times_out_after_10s() {
        // Act
        let settings = load(required()).unwrap();

        // Assert
        assert_eq!(settings.jwks_refresh(), Duration::from_secs(60));
        assert_eq!(settings.request_timeout(), Duration::from_secs(10));
    }

    #[rstest]
    #[case::jwks_refresh("jwks_refresh_secs")]
    #[case::request_timeout("request_timeout_secs")]
    fn deserialize_zero_seconds_is_rejected(#[case] key: &str) {
        // Act
        let result = load(required().set_override(key, "0").unwrap());

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn address_bare_ipv6_host_is_bracketed() {
        // Act
        let address = load(required()).unwrap().address();

        // Assert
        assert_eq!(address, "[::1]:8080");
    }
}
