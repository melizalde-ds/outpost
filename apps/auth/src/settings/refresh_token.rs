use serde::{Deserialize, Deserializer, de::Error};

const MAX_SECS: u64 = 365 * 24 * 60 * 60;

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct RefreshTokenSettings {
    #[serde(deserialize_with = "one_second_to_a_year")]
    pub idle_timeout_secs: u64,
    #[serde(deserialize_with = "one_second_to_a_year")]
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

fn one_second_to_a_year<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
    let secs = u64::deserialize(deserializer)?;
    if (1..=MAX_SECS).contains(&secs) {
        Ok(secs)
    } else {
        Err(D::Error::custom(format!(
            "must be between 1 and {MAX_SECS} seconds (365 days), got {secs}"
        )))
    }
}

#[cfg(test)]
mod tests {
    use config::{Config, ConfigError};
    use rstest::rstest;

    use super::*;

    const DAY_SECS: u64 = 24 * 60 * 60;

    fn load(key: &str, value: &str) -> Result<RefreshTokenSettings, ConfigError> {
        Config::builder()
            .set_override(key, value)?
            .build()?
            .try_deserialize()
    }

    #[test]
    fn deserialize_nothing_set_ends_sessions_after_7_idle_days_and_30_days_total() {
        // Act
        let settings: RefreshTokenSettings = serde_json::from_str("{}").unwrap();

        // Assert
        assert_eq!(settings.idle_timeout_secs, 7 * DAY_SECS);
        assert_eq!(settings.max_age_secs, 30 * DAY_SECS);
    }

    #[rstest]
    #[case::idle_timeout_zero("idle_timeout_secs", "0")]
    #[case::idle_timeout_over_a_year("idle_timeout_secs", "31536001")]
    #[case::max_age_zero("max_age_secs", "0")]
    #[case::max_age_over_a_year("max_age_secs", "31536001")]
    fn deserialize_outside_one_second_to_a_year_is_rejected(
        #[case] key: &str,
        #[case] value: &str,
    ) {
        // Act
        let result = load(key, value);

        // Assert
        assert!(result.is_err());
    }

    #[rstest]
    #[case::idle_timeout_one_second("idle_timeout_secs", "1")]
    #[case::idle_timeout_a_year("idle_timeout_secs", "31536000")]
    #[case::max_age_one_second("max_age_secs", "1")]
    #[case::max_age_a_year("max_age_secs", "31536000")]
    fn deserialize_at_the_limits_is_accepted(#[case] key: &str, #[case] value: &str) {
        // Act
        let result = load(key, value);

        // Assert
        assert!(result.is_ok());
    }
}
