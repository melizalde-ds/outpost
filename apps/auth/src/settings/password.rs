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

#[cfg(test)]
mod tests {
    use super::*;

    const PEPPER: &str = "pepper-value";

    #[test]
    fn password_settings_debug_hides_the_pepper() {
        // Arrange
        let settings = PasswordSettings {
            pepper: SecretString::from(PEPPER),
            concurrency_limit: 1,
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
        assert_eq!(settings.concurrency_limit, 10);
    }
}
