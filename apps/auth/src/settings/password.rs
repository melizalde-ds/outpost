use secrecy::SecretString;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct PasswordSettings {
    pub pepper: SecretString,
    #[serde(default = "default_concurrency_limit")]
    pub concurrency_limit: usize,
}

fn default_concurrency_limit() -> usize {
    10
}
