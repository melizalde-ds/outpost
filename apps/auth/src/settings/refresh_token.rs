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

#[cfg(test)]
mod tests {
    use super::*;

    const DAY_SECS: u64 = 24 * 60 * 60;

    #[test]
    fn deserialize_nothing_set_ends_sessions_after_7_idle_days_and_30_days_total() {
        // Act
        let settings: RefreshTokenSettings = serde_json::from_str("{}").unwrap();

        // Assert
        assert_eq!(settings.idle_timeout_secs, 7 * DAY_SECS);
        assert_eq!(settings.max_age_secs, 30 * DAY_SECS);
    }
}
