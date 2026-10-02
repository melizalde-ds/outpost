use anyhow::bail;
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
    service::SigningService,
};

#[derive(Debug)]
pub struct WeatherSigningService {
    pub client: KeyServiceClient<Channel>,
}

impl WeatherSigningService {
    pub fn new(channel: Channel) -> Self {
        Self {
            client: KeyServiceClient::new(channel),
        }
    }
}

#[async_trait]
impl SigningService for WeatherSigningService {
    async fn sign(&self, sub: Uuid) -> anyhow::Result<String> {
        let mut stub = self.client.clone();
        let response = stub
            .sign_token(SignTokenRequest {
                subject: sub.to_string(),
            })
            .await?
            .into_inner();

        let token = response.token;

        Ok(token)
    }

    async fn get_verification_keys(&self) -> anyhow::Result<Vec<super::PublicKey>> {
        let mut stub = self.client.clone();
        let response = stub
            .get_verification_keys(GetVerificationKeysRequest {})
            .await?
            .into_inner();

        let keys = response.keys;
        let public_keys = keys
            .into_iter()
            .filter_map(|k| {
                super::PublicKey::try_from(k)
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

impl TryFrom<VerificationKey> for super::PublicKey {
    type Error = anyhow::Error;

    fn try_from(value: VerificationKey) -> Result<Self, Self::Error> {
        let kid = Uuid::parse_str(&value.key_id)?;
        if value.public_key.len() != 32 {
            bail!(
                "key {kid}: expected 32 bytes, got {}",
                value.public_key.len()
            );
        }
        let key = URL_SAFE_NO_PAD.encode(&value.public_key);
        let Some(timestamp) = value.expires_at else {
            bail!("Verification key is missing expiration time");
        };

        let Some(exp) = DateTime::from_timestamp(timestamp.seconds, timestamp.nanos as u32) else {
            bail!("Failed to create DateTime from timestamp");
        };

        Ok(super::PublicKey { kid, key, exp })
    }
}
