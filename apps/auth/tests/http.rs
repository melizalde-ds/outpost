mod common;

use common::{
    auth::{Auth, JWKS, PASSWORD},
    http,
    key_stub::{AUDIENCE, ISSUER, KeyStubConfig},
};
use hyper::{StatusCode, header};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, jwk::JwkSet};
use pretty_assertions::assert_eq;
use serde_json::{Value, json};
use tonic::Code;

const ALICE: &str = "alice";
const WRONG_PASSWORD: &str = "wrong horse";
const RELOAD_EVERY_SECOND: (&str, &str) = ("AUTH__HTTP__JWKS_REFRESH_SECS", "1");

fn credentials(password: &str) -> Value {
    json!({ "username": ALICE, "password": password })
}

fn refresh_body(refresh_token: &str) -> Value {
    json!({ "refresh_token": refresh_token })
}

async fn http_sign_up(auth: &Auth) -> Value {
    let reply = http::post(auth.http_addr, "/v1/auth/register", credentials(PASSWORD)).await;
    reply.body["tokens"].clone()
}

#[tokio::test]
async fn http_register_returns_an_access_token_that_verifies_with_the_jwks_key() {
    // Arrange
    let auth = Auth::start().await;

    // Act
    let tokens = http_sign_up(&auth).await;

    // Assert
    let access_token = tokens["access_token"].as_str().unwrap();
    let kid = jsonwebtoken::decode_header(access_token)
        .unwrap()
        .kid
        .unwrap();
    let jwks: JwkSet = serde_json::from_value(http::get(auth.http_addr, JWKS).await.body).unwrap();
    let mut validation = Validation::new(Algorithm::EdDSA);
    validation.set_audience(&[AUDIENCE]);
    validation.set_issuer(&[ISSUER]);
    let verified = jsonwebtoken::decode::<Value>(
        access_token,
        &DecodingKey::from_jwk(jwks.find(&kid).unwrap()).unwrap(),
        &validation,
    );
    assert!(verified.is_ok());
}

#[tokio::test]
async fn http_refresh_after_http_logout_returns_401() {
    // Arrange
    let auth = Auth::start().await;
    let tokens = http_sign_up(&auth).await;
    let refresh_token = tokens["refresh_token"].as_str().unwrap();
    http::post(
        auth.http_addr,
        "/v1/auth/logout",
        refresh_body(refresh_token),
    )
    .await;

    // Act
    let reply = http::post(
        auth.http_addr,
        "/v1/auth/refresh",
        refresh_body(refresh_token),
    )
    .await;

    // Assert
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn http_refresh_of_a_grpc_session_returns_new_tokens() {
    // Arrange
    let auth = Auth::start().await;
    let tokens = auth.sign_up(ALICE).await;

    // Act
    let reply = http::post(
        auth.http_addr,
        "/v1/auth/refresh",
        refresh_body(&tokens.refresh_token),
    )
    .await;

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
    assert!(reply.body["tokens"]["refresh_token"].is_string());
}

#[tokio::test]
async fn http_login_wrong_password_returns_401_problem_with_a_challenge() {
    // Arrange
    let auth = Auth::start().await;
    auth.sign_up(ALICE).await;

    // Act
    let reply = http::post(
        auth.http_addr,
        "/v1/auth/login",
        credentials(WRONG_PASSWORD),
    )
    .await;

    // Assert
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
    assert_eq!(
        reply.headers[header::CONTENT_TYPE],
        "application/problem+json"
    );
    assert_eq!(reply.headers[header::WWW_AUTHENTICATE], "Bearer");
}

#[tokio::test]
async fn jwks_keeps_answering_while_the_key_provider_is_down() {
    // Arrange
    let config = KeyStubConfig::default();
    let key_id = config.key_id.to_string();
    let auth = Auth::start_with(config, &[RELOAD_EVERY_SECOND]).await;
    auth.wait_until_jwks_loaded().await;
    auth.key_provider.set_list_error(Some(Code::Unavailable));
    // Two more calls: by the second, auth has handled the first failure.
    let calls = auth.key_provider.key_list_calls();
    auth.key_provider.wait_for_key_list_calls(calls + 2).await;

    // Act
    let reply = http::get(auth.http_addr, JWKS).await;

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
    let jwks: JwkSet = serde_json::from_value(reply.body).unwrap();
    assert!(jwks.find(&key_id).is_some());
}

#[tokio::test]
async fn jwks_before_the_key_provider_ever_answers_returns_503() {
    // Arrange
    let config = KeyStubConfig {
        list_error: Some(Code::Unavailable),
        ..KeyStubConfig::default()
    };
    let auth = Auth::start_with(config, &[RELOAD_EVERY_SECOND]).await;
    auth.key_provider.wait_for_key_list_calls(2).await;

    // Act
    let reply = http::get(auth.http_addr, JWKS).await;

    // Assert
    assert_eq!(reply.status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(reply.headers.contains_key(header::RETRY_AFTER));
}
