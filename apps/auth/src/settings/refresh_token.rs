use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RefreshTokenSettings {
    #[serde(default = "default_idle_timeout_secs")]
    pub idle_timeout_secs: u64,
    #[serde(default = "default_max_age_secs")]
    pub max_age_secs: u64,
}

impl Default for RefreshTokenSettings {
    fn default() -> Self {
        Self {
            idle_timeout_secs: default_idle_timeout_secs(),
            max_age_secs: default_max_age_secs(),
        }
    }
}

fn default_idle_timeout_secs() -> u64 {
    604800
}

fn default_max_age_secs() -> u64 {
    2592000
}
