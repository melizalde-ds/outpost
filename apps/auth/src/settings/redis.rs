use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RedisSettings {
    pub host: String,
    pub port: u16,
}

impl Default for RedisSettings {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 6379,
        }
    }
}

impl RedisSettings {
    pub fn get_redis_address(&self) -> String {
        format!("redis://{}:{}", self.host, self.port)
    }
}
