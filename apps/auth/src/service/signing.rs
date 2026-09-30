use tonic::transport::Channel;
use uuid::Uuid;

use crate::proto::key::v1::key_service_client::KeyServiceClient;

#[derive(Debug)]
pub struct SigningService {
    pub client: KeyServiceClient<Channel>,
}

impl SigningService {
    pub fn new(client: KeyServiceClient<Channel>) -> Self {
        Self { client }
    }

    pub async fn sign(&mut self, payload: Uuid) -> anyhow::Result<()> {
        todo!()
    }
}
