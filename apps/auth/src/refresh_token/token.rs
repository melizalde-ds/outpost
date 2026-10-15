use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::repository::{TokenFamilyId, UserId};

pub struct RefreshToken {
    pub user_id: UserId,
    pub hash: [u8; 32],
    pub exp: DateTime<Utc>,
    pub family_exp: DateTime<Utc>,
}

impl RefreshToken {
    pub fn generate(
        user_id: UserId,
        family_id: TokenFamilyId,
        exp: DateTime<Utc>,
        family_exp: DateTime<Utc>,
    ) -> Result<(String, Self), getrandom::Error> {
        let mut secret = [0u8; 32];
        getrandom::fill(&mut secret)?;
        let token = format!(
            "{}.{}",
            family_id.0.simple(),
            URL_SAFE_NO_PAD.encode(secret)
        );
        let hash = hash_token(&token);

        Ok((
            token,
            Self {
                user_id,
                hash,
                exp,
                family_exp,
            },
        ))
    }

    pub fn parse(token: &str) -> Option<(TokenFamilyId, [u8; 32])> {
        let (family_id, secret) = token.split_once('.')?;
        let family_id = Uuid::parse_str(family_id).ok()?;
        let secret = URL_SAFE_NO_PAD.decode(secret).ok()?;
        if secret.len() != 32 {
            return None;
        }

        Some((family_id.into(), hash_token(token)))
    }
}

fn hash_token(token: &str) -> [u8; 32] {
    Sha256::digest(token.as_bytes()).into()
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use pretty_assertions::{assert_eq, assert_ne};
    use proptest::prelude::*;
    use rstest::rstest;

    use super::*;

    const USER: UserId = UserId(Uuid::from_u128(1));
    const FAMILY: TokenFamilyId = TokenFamilyId(Uuid::from_u128(2));
    const OTHER_FAMILY: TokenFamilyId = TokenFamilyId(Uuid::from_u128(3));

    fn generate() -> (String, RefreshToken) {
        let exp = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        RefreshToken::generate(USER, FAMILY, exp, exp).unwrap()
    }

    #[test]
    fn parse_generated_token_returns_its_family_and_hash() {
        // Arrange
        let (token, refresh_token) = generate();

        // Act
        let parsed = RefreshToken::parse(&token);

        // Assert
        assert_eq!(parsed, Some((FAMILY, refresh_token.hash)));
    }

    #[test]
    fn generate_twice_returns_different_tokens() {
        // Arrange
        let (first, _) = generate();

        // Act
        let (second, _) = generate();

        // Assert
        assert_ne!(first, second);
    }

    #[test]
    fn parse_token_moved_to_another_family_returns_a_different_hash() {
        // Arrange
        let (token, refresh_token) = generate();
        let moved = token.replacen(
            &FAMILY.0.simple().to_string(),
            &OTHER_FAMILY.0.simple().to_string(),
            1,
        );

        // Act
        let parsed = RefreshToken::parse(&moved);

        // Assert
        assert_ne!(parsed.map(|(_, hash)| hash), Some(refresh_token.hash));
    }

    #[rstest]
    #[case::empty("")]
    #[case::no_separator("no-separator")]
    #[case::bad_family_id("not-a-uuid.AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA")]
    #[case::bad_encoding("00000000000000000000000000000002.not base64!")]
    #[case::secret_too_short("00000000000000000000000000000002.AAAA")]
    #[case::secret_too_long(
        "00000000000000000000000000000002.AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
    )]
    fn parse_malformed_token_returns_none(#[case] token: &str) {
        // Act
        let parsed = RefreshToken::parse(token);

        // Assert
        assert_eq!(parsed, None);
    }

    #[test]
    fn parse_well_formed_token_returns_some() {
        // Act
        let parsed = RefreshToken::parse(
            "00000000000000000000000000000002.AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        );

        // Assert
        assert!(parsed.is_some());
    }

    proptest! {
        #[test]
        fn parse_any_input_does_not_panic(input in ".*") {
            // Act
            let _ = RefreshToken::parse(&input);
        }
    }
}
