use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::DateTime;
use tonic::{async_trait, transport::Channel};
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
    pub client: KeyServiceClient<Channel>,
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
                        error!("Failed to convert verification key: {}", e);
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
