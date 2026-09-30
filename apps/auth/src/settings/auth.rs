use secrecy::SecretString;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AuthSettings {
    pub password_pepper: SecretString,
    pub concurrency_limit: usize,
}

impl Default for AuthSettings {
    fn default() -> Self {
        Self {
            password_pepper: "Hello, Auth!".into(),
            concurrency_limit: 10,
        }
    }
}
