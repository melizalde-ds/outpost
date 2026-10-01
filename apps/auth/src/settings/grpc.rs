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
    pub connect_timeout: u64,
    pub timeout: u64,
}

impl Default for GrpcSettings {
    fn default() -> Self {
        Self {
            server: GrpcEndpoint {
                host: "[::]".to_string(),
                port: 50051,
                channel_settings: ChannelSettings::default(),
            },
            key_service_client: GrpcEndpoint {
                host: "[::1]".to_string(),
                port: 50052,
                channel_settings: ChannelSettings::default(),
            },
        }
    }
}

impl Default for ChannelSettings {
    fn default() -> Self {
        Self {
            connect_timeout: 2,
            timeout: 3,
        }
    }
}

impl GrpcEndpoint {
    pub fn get_grpc_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub fn client_uri(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }
}
