use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ServerSettings {
    pub host: String,
    pub port: u16,
}

impl Default for ServerSettings {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 8080,
        }
    }
}

impl ServerSettings {
    pub fn get_server_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
