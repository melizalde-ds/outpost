use std::num::NonZeroUsize;

use secrecy::SecretString;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct PasswordSettings {
    pub pepper: SecretString,
    #[serde(default = "default_concurrency_limit")]
    pub concurrency_limit: NonZeroUsize,
}

fn default_concurrency_limit() -> NonZeroUsize {
    NonZeroUsize::new(10).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    const PEPPER: &str = "pepper-value";

    #[test]
    fn password_settings_debug_hides_the_pepper() {
        // Arrange
        let settings = PasswordSettings {
            pepper: SecretString::from(PEPPER),
            concurrency_limit: NonZeroUsize::MIN,
        };

        // Act
        let output = format!("{settings:?}");

        // Assert
        assert!(!output.contains(PEPPER));
    }

    #[test]
    fn concurrency_limit_defaults_to_10() {
        // Act
        let settings: PasswordSettings =
            serde_json::from_str(r#"{ "pepper": "pepper-value" }"#).unwrap();

        // Assert
        assert_eq!(settings.concurrency_limit.get(), 10);
    }

    #[test]
    fn concurrency_limit_of_zero_is_rejected() {
        // Act
        let result = serde_json::from_str::<PasswordSettings>(
            r#"{ "pepper": "pepper-value", "concurrency_limit": 0 }"#,
        );

        // Assert
        assert!(result.is_err());
    }
}
