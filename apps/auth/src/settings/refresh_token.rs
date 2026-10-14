use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct RefreshTokenSettings {
    pub idle_timeout_secs: u64,
    pub max_age_secs: u64,
}

impl Default for RefreshTokenSettings {
    fn default() -> Self {
        Self {
            idle_timeout_secs: 604800,
            max_age_secs: 2592000,
        }
    }
}
