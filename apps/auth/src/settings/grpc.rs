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
    pub channel_settings: Option<ChannelSettings>,
}

#[derive(Debug, Deserialize)]
pub struct ChannelSettings {
    pub connect_timeout: u64,
    pub timeout: u64,
}

impl Default for GrpcSettings {
    fn default() -> Self {
        Self {
            server: GrpcEndpoint {
                host: "localhost".to_string(),
                port: 50051,
                channel_settings: None,
            },
            key_service_client: GrpcEndpoint {
                host: "localhost".to_string(),
                port: 50052,
                channel_settings: None,
            },
        }
    }
}

impl GrpcEndpoint {
    pub fn get_grpc_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
