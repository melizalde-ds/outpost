use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use prost_types::Timestamp;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tonic::{Code, Request, Status, transport::Channel};
use uuid::Uuid;

use super::{
    admin,
    app::App,
    fresh_db,
    key_stub::{KeyStub, KeyStubConfig},
    proto::auth::v1::{
        LoginRequest, LogoutAllRequest, LogoutRequest, RefreshTokenRequest, RegisterRequest,
        TokenPair, auth_service_client::AuthServiceClient,
    },
    redis,
};

pub const PASSWORD: &str = "correct horse";

pub struct Auth {
    pub key_provider: KeyStub,
    admin: PgPool,
    client: AuthServiceClient<Channel>,
    _app: App,
}

impl Auth {
    pub async fn start() -> Self {
        Self::start_with(KeyStubConfig::default(), &[]).await
    }

    pub async fn start_with(config: KeyStubConfig, settings: &[(&str, &str)]) -> Self {
        let db = fresh_db().await;
        let key_provider = KeyStub::start(config).await;
        let app = App::start_with(&db, key_provider.addr, settings).await;
        Self {
            key_provider,
            admin: admin(&db).await,
            client: app.client(),
            _app: app,
        }
    }

    pub async fn register(&self, username: &str, password: &str) -> Result<TokenPair, Status> {
        let request = RegisterRequest {
            username: username.into(),
            password: password.into(),
        };
        let response = self.client.clone().register(Request::new(request)).await?;
        Ok(response.into_inner().tokens.unwrap())
    }

    pub async fn login(&self, username: &str, password: &str) -> Result<TokenPair, Status> {
        let request = LoginRequest {
            username: username.into(),
            password: password.into(),
        };
        let response = self.client.clone().login(Request::new(request)).await?;
        Ok(response.into_inner().tokens.unwrap())
    }

    pub async fn refresh(&self, refresh_token: &str) -> Result<TokenPair, Status> {
        let request = RefreshTokenRequest {
            refresh_token: refresh_token.into(),
        };
        let response = self.client.clone().refresh_token(request).await?;
        Ok(response.into_inner().tokens.unwrap())
    }

    pub async fn logout(&self, refresh_token: &str) -> Result<(), Status> {
        let request = LogoutRequest {
            refresh_token: refresh_token.into(),
        };
        self.client.clone().logout(request).await.map(|_| ())
    }

    pub async fn logout_all(&self, refresh_token: &str) -> Result<(), Status> {
        let request = LogoutAllRequest {
            refresh_token: refresh_token.into(),
        };
        self.client.clone().logout_all(request).await.map(|_| ())
    }

    pub async fn sign_up(&self, username: &str) -> TokenPair {
        self.register(username, PASSWORD).await.unwrap()
    }

    pub async fn sign_in(&self, username: &str) -> TokenPair {
        self.login(username, PASSWORD).await.unwrap()
    }

    pub async fn user_id(&self, username: &str) -> Uuid {
        sqlx::query_scalar("SELECT user_id FROM auth WHERE username = $1")
            .bind(username)
            .fetch_one(&self.admin)
            .await
            .unwrap()
    }

    pub async fn usernames(&self) -> Vec<String> {
        sqlx::query_scalar("SELECT username FROM auth ORDER BY username")
            .fetch_all(&self.admin)
            .await
            .unwrap()
    }

    pub async fn password_hash(&self, username: &str) -> String {
        sqlx::query_scalar("SELECT password_hash FROM auth WHERE username = $1")
            .bind(username)
            .fetch_one(&self.admin)
            .await
            .unwrap()
    }

    pub async fn switch_off(&self, username: &str) {
        sqlx::query("UPDATE auth SET active = false WHERE username = $1")
            .bind(username)
            .execute(&self.admin)
            .await
            .unwrap();
    }

    /// When the user's only session ends, however busy it is.
    pub async fn session_limit(&self, username: &str) -> DateTime<Utc> {
        sqlx::query_scalar(
            "SELECT w.expires_at FROM token_family_window w
             JOIN token_family f USING (family_id) JOIN auth a USING (user_id)
             WHERE a.username = $1",
        )
        .bind(username)
        .fetch_one(&self.admin)
        .await
        .unwrap()
    }

    pub async fn pass_session_limit(&self, username: &str) {
        sqlx::query(
            "UPDATE token_family_window w SET expires_at = now() - interval '1 second'
             FROM token_family f JOIN auth a USING (user_id)
             WHERE w.family_id = f.family_id AND a.username = $1",
        )
        .bind(username)
        .execute(&self.admin)
        .await
        .unwrap();
    }

    pub async fn revocation_reasons(&self, username: &str) -> Vec<String> {
        sqlx::query_scalar(
            "SELECT r.reason FROM token_family_revocation r
             JOIN token_family f USING (family_id) JOIN auth a USING (user_id)
             WHERE a.username = $1 ORDER BY r.reason",
        )
        .bind(username)
        .fetch_all(&self.admin)
        .await
        .unwrap()
    }
}

/// Where Redis keeps a refresh token's session.
pub fn session_key(refresh_token: &str) -> String {
    let hash = Sha256::digest(refresh_token.as_bytes());
    format!("refresh_token:{}", URL_SAFE_NO_PAD.encode(hash))
}

/// Where apps read a public key, as the README says.
pub fn jwk_key(kid: &str) -> String {
    format!("jwk:{kid}")
}

/// When a Redis key expires, in Unix milliseconds. Negative if it has no expiry or is missing.
pub async fn redis_expiry(key: &str) -> i64 {
    ::redis::cmd("PEXPIRETIME")
        .arg(key)
        .query_async(&mut redis().await)
        .await
        .unwrap()
}

pub async fn redis_has(key: &str) -> bool {
    ::redis::cmd("EXISTS")
        .arg(key)
        .query_async(&mut redis().await)
        .await
        .unwrap()
}

pub async fn redis_json(key: &str) -> serde_json::Value {
    let json: String = ::redis::cmd("JSON.GET")
        .arg(key)
        .query_async(&mut redis().await)
        .await
        .unwrap();
    serde_json::from_str(&json).unwrap()
}

pub fn millis(timestamp: &Timestamp) -> i64 {
    timestamp.seconds * 1000 + i64::from(timestamp.nanos) / 1_000_000
}

pub fn micros(timestamp: &Timestamp) -> i64 {
    timestamp.seconds * 1_000_000 + i64::from(timestamp.nanos) / 1000
}

#[derive(serde::Deserialize)]
struct Subject {
    sub: String,
}

/// The user an access token is for. The signature is checked in `signing_keys.rs`.
pub fn subject_of(access_token: &str) -> String {
    jsonwebtoken::dangerous::insecure_decode::<Subject>(access_token)
        .unwrap()
        .claims
        .sub
}

pub fn code_of<T>(result: Result<T, Status>) -> Code {
    result.err().expect("expected an error").code()
}
