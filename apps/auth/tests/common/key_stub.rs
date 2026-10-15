use std::{net::SocketAddr, sync::Arc};

use jsonwebtoken::{Algorithm, EncodingKey, Header, get_current_timestamp};
use prost_types::Timestamp;
use serde::Serialize;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::{Request, Response, Status, transport::Server};
use uuid::Uuid;

use super::proto::key::v1::{
    GetVerificationKeysRequest, GetVerificationKeysResponse, SignTokenRequest, SignTokenResponse,
    VerificationKey,
    key_service_server::{KeyService, KeyServiceServer},
};

const PRIVATE_KEY_PEM: &str = "-----BEGIN PRIVATE KEY-----
MC4CAQAwBQYDK2VwBCIEIAIctKioMG5uGhAWdind+64/DNBHdEPUWsNLhytNzD+N
-----END PRIVATE KEY-----";
pub const PUBLIC_KEY: [u8; 32] = [
    53, 168, 205, 153, 155, 232, 18, 7, 253, 189, 250, 119, 218, 140, 159, 121, 57, 91, 216, 63,
    38, 92, 29, 87, 150, 130, 84, 106, 186, 97, 97, 156,
];

pub const KEY_EXPIRES_AT: i64 = 4_102_444_800;
pub const ISSUER: &str = "Outpost";
pub const AUDIENCE: &str = "Outpost";

#[derive(Clone)]
pub struct KeyStubConfig {
    pub key_id: Uuid,
    pub listed_keys: Vec<VerificationKey>,
}

impl Default for KeyStubConfig {
    fn default() -> Self {
        let key_id = Uuid::now_v7();
        Self {
            key_id,
            listed_keys: vec![verification_key(key_id)],
        }
    }
}

pub fn verification_key(key_id: Uuid) -> VerificationKey {
    VerificationKey {
        key_id: key_id.to_string(),
        public_key: PUBLIC_KEY.to_vec(),
        expires_at: Some(Timestamp {
            seconds: KEY_EXPIRES_AT,
            nanos: 0,
        }),
    }
}

#[derive(Serialize)]
struct Claims<'a> {
    sub: &'a str,
    iss: &'a str,
    aud: &'a str,
    iat: u64,
    exp: u64,
}

struct Stub {
    config: KeyStubConfig,
    signing_key: EncodingKey,
}

#[tonic::async_trait]
impl KeyService for Stub {
    async fn sign_token(
        &self,
        request: Request<SignTokenRequest>,
    ) -> Result<Response<SignTokenResponse>, Status> {
        let subject = request.into_inner().subject;
        let mut header = Header::new(Algorithm::EdDSA);
        header.kid = Some(self.config.key_id.to_string());
        let now = get_current_timestamp();
        let claims = Claims {
            sub: &subject,
            iss: ISSUER,
            aud: AUDIENCE,
            iat: now,
            exp: now + 300,
        };
        let token = jsonwebtoken::encode(&header, &claims, &self.signing_key).unwrap();

        Ok(Response::new(SignTokenResponse {
            token,
            key_id: self.config.key_id.to_string(),
            verification_key: None,
        }))
    }

    async fn get_verification_keys(
        &self,
        _request: Request<GetVerificationKeysRequest>,
    ) -> Result<Response<GetVerificationKeysResponse>, Status> {
        Ok(Response::new(GetVerificationKeysResponse {
            keys: self.config.listed_keys.clone(),
        }))
    }
}

pub struct KeyStub {
    pub addr: SocketAddr,
}

impl KeyStub {
    pub async fn start(config: KeyStubConfig) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let stub = Arc::new(Stub {
            config,
            signing_key: EncodingKey::from_ed_pem(PRIVATE_KEY_PEM.as_bytes()).unwrap(),
        });
        tokio::spawn(
            Server::builder()
                .add_service(KeyServiceServer::from_arc(stub))
                .serve_with_incoming(TcpListenerStream::new(listener)),
        );
        Self { addr }
    }
}
