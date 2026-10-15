use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::DateTime;
use tonic::transport::Channel;
use tracing::error;
use uuid::Uuid;

use crate::{
    proto::key::v1::{
        GetVerificationKeysRequest, SignTokenRequest, VerificationKey,
        key_service_client::KeyServiceClient,
    },
    signing::{PublicKey, SignedToken, SigningError, SigningService},
};

#[derive(Debug)]
pub struct KeySigningService {
    client: KeyServiceClient<Channel>,
}

impl KeySigningService {
    pub fn new(channel: Channel) -> Self {
        Self {
            client: KeyServiceClient::new(channel),
        }
    }
}

#[async_trait]
impl SigningService for KeySigningService {
    async fn sign(&self, sub: Uuid) -> Result<SignedToken, SigningError> {
        let mut stub = self.client.clone();
        let response = stub
            .sign_token(SignTokenRequest {
                subject: sub.to_string(),
            })
            .await?
            .into_inner();

        let key_id = Uuid::parse_str(&response.key_id)?;
        let new_key = response
            .verification_key
            .map(PublicKey::try_from)
            .transpose()?;

        Ok(SignedToken {
            token: response.token,
            key_id,
            new_key,
        })
    }

    async fn get_verification_keys(&self) -> Result<Vec<PublicKey>, SigningError> {
        let mut stub = self.client.clone();
        let response = stub
            .get_verification_keys(GetVerificationKeysRequest {})
            .await?
            .into_inner();

        let keys = response.keys;
        let public_keys = keys
            .into_iter()
            .filter_map(|k| {
                PublicKey::try_from(k)
                    .map_err(|e| {
                        error!(error = %e, "failed to convert verification key");
                        e
                    })
                    .ok()
            })
            .collect();

        Ok(public_keys)
    }
}

impl TryFrom<VerificationKey> for PublicKey {
    type Error = SigningError;

    fn try_from(value: VerificationKey) -> Result<Self, Self::Error> {
        let kid = Uuid::parse_str(&value.key_id)?;
        if value.public_key.len() != 32 {
            return Err(SigningError::BadKeyLength(kid, value.public_key.len()));
        }
        let key = URL_SAFE_NO_PAD.encode(&value.public_key);
        let Some(timestamp) = value.expires_at else {
            return Err(SigningError::MissingExpiry(kid));
        };

        let Some(exp) = DateTime::from_timestamp(timestamp.seconds, timestamp.nanos as u32) else {
            return Err(SigningError::BadExpiry(kid));
        };

        Ok(PublicKey { kid, key, exp })
    }
}

#[cfg(test)]
mod tests {
    use prost_types::Timestamp;
    use rstest::rstest;

    use super::*;

    const KEY_ID: &str = "00000000-0000-0000-0000-000000000001";
    const EXPIRES_AT: i64 = 1_767_225_600;

    fn verification_key(public_key: Vec<u8>, expires_at: Option<Timestamp>) -> VerificationKey {
        VerificationKey {
            key_id: KEY_ID.into(),
            public_key,
            expires_at,
        }
    }

    fn expiry() -> Option<Timestamp> {
        Some(Timestamp {
            seconds: EXPIRES_AT,
            nanos: 0,
        })
    }

    #[test]
    fn try_from_valid_key_returns_encoded_public_key() {
        // Arrange
        let key = verification_key(vec![0; 32], expiry());

        // Act
        let public_key = PublicKey::try_from(key).unwrap();

        // Assert
        assert_eq!(public_key.kid.to_string(), KEY_ID);
        assert_eq!(public_key.key, URL_SAFE_NO_PAD.encode([0; 32]));
        assert_eq!(public_key.exp.timestamp(), EXPIRES_AT);
    }

    #[rstest]
    #[case::empty(0)]
    #[case::too_short(31)]
    #[case::too_long(33)]
    fn try_from_key_with_wrong_length_returns_bad_key_length(#[case] length: usize) {
        // Arrange
        let key = verification_key(vec![0; length], expiry());

        // Act
        let result = PublicKey::try_from(key);

        // Assert
        assert!(matches!(result, Err(SigningError::BadKeyLength(_, l)) if l == length));
    }

    #[test]
    fn try_from_key_without_expiry_returns_missing_expiry() {
        // Arrange
        let key = verification_key(vec![0; 32], None);

        // Act
        let result = PublicKey::try_from(key);

        // Assert
        assert!(matches!(result, Err(SigningError::MissingExpiry(_))));
    }

    #[test]
    fn try_from_key_with_invalid_id_returns_bad_key_id() {
        // Arrange
        let key = VerificationKey {
            key_id: "not-a-uuid".into(),
            ..verification_key(vec![0; 32], expiry())
        };

        // Act
        let result = PublicKey::try_from(key);

        // Assert
        assert!(matches!(result, Err(SigningError::BadKeyId(_))));
    }
}
