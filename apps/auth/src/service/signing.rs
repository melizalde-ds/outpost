use tonic::transport::Channel;
use uuid::Uuid;

use crate::{proto::key::v1::key_service_client::KeyServiceClient, service::SigningService};

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

impl SigningService for WeatherSigningService {
    fn sign(&self, sub: Uuid) -> anyhow::Result<String> {
        todo!()
    }

    fn get_verification_keys(&self) -> anyhow::Result<Vec<super::PublicKey>> {
        todo!()
    }
}
