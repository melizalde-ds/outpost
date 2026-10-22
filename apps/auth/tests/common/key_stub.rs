use std::{
    net::SocketAddr,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use jsonwebtoken::{Algorithm, EncodingKey, Header, get_current_timestamp};
use prost_types::Timestamp;
use serde::Serialize;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::{Code, Request, Response, Status, transport::Server};
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
    pub new_key: Option<VerificationKey>,
    pub sign_error: Option<Code>,
    pub sign_hangs: bool,
    pub list_error: Option<Code>,
}

impl Default for KeyStubConfig {
    fn default() -> Self {
        let key_id = Uuid::now_v7();
        Self {
            key_id,
            listed_keys: vec![verification_key(key_id)],
            new_key: None,
            sign_error: None,
            sign_hangs: false,
            list_error: None,
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
    config: Arc<Mutex<KeyStubConfig>>,
    signing_key: EncodingKey,
    key_list_calls: Arc<AtomicUsize>,
}

#[tonic::async_trait]
impl KeyService for Stub {
    async fn sign_token(
        &self,
        request: Request<SignTokenRequest>,
    ) -> Result<Response<SignTokenResponse>, Status> {
        let config = self.config.lock().unwrap().clone();
        if config.sign_hangs {
            std::future::pending::<()>().await;
        }
        if let Some(code) = config.sign_error {
            return Err(Status::new(code, "key stub failure"));
        }
        let subject = request.into_inner().subject;
        let mut header = Header::new(Algorithm::EdDSA);
        header.kid = Some(config.key_id.to_string());
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
            key_id: config.key_id.to_string(),
            verification_key: config.new_key,
        }))
    }

    async fn get_verification_keys(
        &self,
        _request: Request<GetVerificationKeysRequest>,
    ) -> Result<Response<GetVerificationKeysResponse>, Status> {
        self.key_list_calls.fetch_add(1, Ordering::SeqCst);
        let config = self.config.lock().unwrap().clone();
        if let Some(code) = config.list_error {
            return Err(Status::new(code, "key stub failure"));
        }
        Ok(Response::new(GetVerificationKeysResponse {
            keys: config.listed_keys,
        }))
    }
}

pub struct KeyStub {
    pub addr: SocketAddr,
    config: Arc<Mutex<KeyStubConfig>>,
    key_list_calls: Arc<AtomicUsize>,
}

impl KeyStub {
    pub async fn start(config: KeyStubConfig) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let config = Arc::new(Mutex::new(config));
        let key_list_calls = Arc::new(AtomicUsize::new(0));
        let stub = Arc::new(Stub {
            config: config.clone(),
            signing_key: EncodingKey::from_ed_pem(PRIVATE_KEY_PEM.as_bytes()).unwrap(),
            key_list_calls: key_list_calls.clone(),
        });
        tokio::spawn(
            Server::builder()
                .add_service(KeyServiceServer::from_arc(stub))
                .serve_with_incoming(TcpListenerStream::new(listener)),
        );
        Self {
            addr,
            config,
            key_list_calls,
        }
    }

    pub fn set_sign_error(&self, code: Option<Code>) {
        self.config.lock().unwrap().sign_error = code;
    }

    pub fn set_list_error(&self, code: Option<Code>) {
        self.config.lock().unwrap().list_error = code;
    }

    pub fn key_list_calls(&self) -> usize {
        self.key_list_calls.load(Ordering::SeqCst)
    }

    pub async fn wait_for_key_list_calls(&self, calls: usize) {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
        while self.key_list_calls() < calls {
            assert!(
                tokio::time::Instant::now() < deadline,
                "fewer than {calls} key list calls after 10s"
            );
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }
}
