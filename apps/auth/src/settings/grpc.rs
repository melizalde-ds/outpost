use std::net::{IpAddr, SocketAddr};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GrpcSettings {
    pub server: GrpcEndpoint,
    pub key_service_client: GrpcEndpoint,
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

impl GrpcEndpoint {
    pub fn grpc_address(&self) -> String {
        match self.host.parse::<IpAddr>() {
            Ok(ip) => SocketAddr::new(ip, self.port).to_string(),
            Err(_) => format!("{}:{}", self.host, self.port),
        }
    }

    pub fn client_uri(&self) -> String {
        format!("http://{}", self.grpc_address())
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

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
