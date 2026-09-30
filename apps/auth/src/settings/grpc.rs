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
}

impl Default for GrpcSettings {
    fn default() -> Self {
        Self {
            server: GrpcEndpoint {
                host: "localhost".to_string(),
                port: 50051,
            },
            key_service_client: GrpcEndpoint {
                host: "localhost".to_string(),
                port: 50052,
            },
        }
    }
}

impl GrpcEndpoint {
    pub fn get_grpc_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
