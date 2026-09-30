use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GrpcSettings {
    pub host: String,
    pub port: u16,
}

impl Default for GrpcSettings {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 50051,
        }
    }
}

impl GrpcSettings {
    pub fn get_grpc_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
