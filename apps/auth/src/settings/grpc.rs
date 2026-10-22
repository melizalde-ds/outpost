use std::{
    net::{IpAddr, SocketAddr},
    num::NonZeroU64,
    time::Duration,
};

use serde::Deserialize;

use super::http::default_request_timeout_secs;

#[derive(Debug, Deserialize)]
pub struct GrpcSettings {
    pub server: GrpcEndpoint,
    pub key_service_client: GrpcEndpoint,
    #[serde(default = "default_request_timeout_secs")]
    pub request_timeout_secs: NonZeroU64,
}

impl GrpcSettings {
    pub fn request_timeout(&self) -> Duration {
        Duration::from_secs(self.request_timeout_secs.get())
    }
}

#[derive(Debug, Deserialize)]
pub struct GrpcEndpoint {
    pub host: String,
    pub port: u16,
    #[serde(default)]
    pub channel_settings: ChannelSettings,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct ChannelSettings {
    pub connect_timeout_secs: u64,
    pub timeout_secs: u64,
}

impl Default for ChannelSettings {
    fn default() -> Self {
        Self {
            connect_timeout_secs: 2,
            timeout_secs: 3,
        }
    }
}

pub(super) fn host_port(host: &str, port: u16) -> String {
    match host.parse::<IpAddr>() {
        Ok(ip) => SocketAddr::new(ip, port).to_string(),
        Err(_) => format!("{host}:{port}"),
    }
}

impl GrpcEndpoint {
    pub fn grpc_address(&self) -> String {
        host_port(&self.host, self.port)
    }

    pub fn client_uri(&self) -> String {
        format!("http://{}", self.grpc_address())
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    const ENDPOINTS: &str = r#""server": { "host": "::1", "port": 50051 },
        "key_service_client": { "host": "::1", "port": 5210 }"#;

    #[test]
    fn deserialize_nothing_set_times_out_after_10s() {
        // Act
        let settings: GrpcSettings = serde_json::from_str(&format!("{{ {ENDPOINTS} }}")).unwrap();

        // Assert
        assert_eq!(settings.request_timeout(), Duration::from_secs(10));
    }

    #[test]
    fn deserialize_zero_request_timeout_is_rejected() {
        // Act
        let result = serde_json::from_str::<GrpcSettings>(&format!(
            r#"{{ {ENDPOINTS}, "request_timeout_secs": 0 }}"#
        ));

        // Assert
        assert!(result.is_err());
    }

    fn endpoint(host: &str) -> GrpcEndpoint {
        GrpcEndpoint {
            host: host.into(),
            port: 50051,
            channel_settings: ChannelSettings::default(),
        }
    }

    #[rstest]
    #[case::ipv4("127.0.0.1", "127.0.0.1:50051")]
    #[case::ipv6("::1", "[::1]:50051")]
    #[case::hostname("key-provider", "key-provider:50051")]
    fn grpc_address_host_returns_host_and_port(#[case] host: &str, #[case] expected: &str) {
        // Act
        let address = endpoint(host).grpc_address();

        // Assert
        assert_eq!(address, expected);
    }

    #[rstest]
    #[case::ipv4("127.0.0.1", "http://127.0.0.1:50051")]
    #[case::ipv6("::1", "http://[::1]:50051")]
    #[case::hostname("key-provider", "http://key-provider:50051")]
    fn client_uri_host_returns_http_uri(#[case] host: &str, #[case] expected: &str) {
        // Act
        let uri = endpoint(host).client_uri();

        // Assert
        assert_eq!(uri, expected);
    }
}
