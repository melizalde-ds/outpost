mod common;

use common::{
    app::{App, free_address},
    auth::{Auth, PASSWORD, code_of, jwk_key, redis_delete, redis_expiry, redis_has, redis_json},
    fresh_db,
    key_stub::{AUDIENCE, ISSUER, KEY_EXPIRES_AT, KeyStubConfig, verification_key},
    proto::auth::v1::RegisterRequest,
};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, jwk::Jwk};
use pretty_assertions::assert_eq;
use serde_json::json;
use tonic::Code;
use uuid::Uuid;

const ALICE: &str = "alice";
const BOB: &str = "bob";
const PUBLIC_KEY_X: &str = "NajNmZvoEgf9vfp32oyfeTlb2D8mXB1XloJUarphYZw";

fn config_listing(other_key: Uuid) -> KeyStubConfig {
    let config = KeyStubConfig::default();
    KeyStubConfig {
        listed_keys: vec![verification_key(config.key_id), verification_key(other_key)],
        ..config
    }
}

#[tokio::test]
async fn access_token_verifies_with_the_public_key_apps_read_from_redis() {
    // Arrange
    let auth = Auth::start().await;

    // Act
    let tokens = auth.register(ALICE, PASSWORD).await.unwrap();

    // Assert
    let kid = jsonwebtoken::decode_header(&tokens.access_token)
        .unwrap()
        .kid
        .unwrap();
    let jwk: Jwk = serde_json::from_value(redis_json(&jwk_key(&kid)).await).unwrap();
    let mut validation = Validation::new(Algorithm::EdDSA);
    validation.set_audience(&[AUDIENCE]);
    validation.set_issuer(&[ISSUER]);
    let verified = jsonwebtoken::decode::<serde_json::Value>(
        &tokens.access_token,
        &DecodingKey::from_jwk(&jwk).unwrap(),
        &validation,
    );
    assert!(verified.is_ok());
}

#[tokio::test]
async fn first_sign_in_saves_every_listed_key_as_an_ed25519_jwk() {
    // Arrange
    let other_key = Uuid::now_v7();
    let auth = Auth::start_with(config_listing(other_key), &[]).await;

    // Act
    auth.register(ALICE, PASSWORD).await.unwrap();

    // Assert
    assert_eq!(
        redis_json(&jwk_key(&other_key.to_string())).await,
        json!({
            "kty": "OKP",
            "crv": "Ed25519",
            "kid": other_key.to_string(),
            "alg": "EdDSA",
            "use": "sig",
            "x": PUBLIC_KEY_X,
        })
    );
}

#[tokio::test]
async fn saved_key_disappears_when_the_key_expires() {
    // Arrange
    let other_key = Uuid::now_v7();
    let auth = Auth::start_with(config_listing(other_key), &[]).await;

    // Act
    auth.register(ALICE, PASSWORD).await.unwrap();

    // Assert
    assert_eq!(
        redis_expiry(&jwk_key(&other_key.to_string())).await,
        KEY_EXPIRES_AT * 1000
    );
}

#[tokio::test]
async fn key_provider_down_at_startup_auth_still_starts_and_sign_up_returns_unavailable() {
    // Arrange
    let db = fresh_db().await;
    let nothing_listening = free_address();
    let app = App::start(&db, nothing_listening).await;
    let request = RegisterRequest {
        username: ALICE.into(),
        password: PASSWORD.into(),
    };

    // Act
    let result = app.client().register(request).await;

    // Assert
    assert_eq!(code_of(result), Code::Unavailable);
}

#[tokio::test]
async fn brand_new_signing_key_is_saved_for_apps() {
    // Arrange
    let new_key = Uuid::now_v7();
    let auth = Auth::start_with(
        KeyStubConfig {
            key_id: new_key,
            new_key: Some(verification_key(new_key)),
            ..KeyStubConfig::default()
        },
        &[],
    )
    .await;

    // Act
    auth.register(ALICE, PASSWORD).await.unwrap();

    // Assert
    assert!(redis_has(&jwk_key(&new_key.to_string())).await);
}

#[tokio::test]
async fn signing_key_missing_from_redis_is_saved_again_on_the_next_sign_in() {
    // Arrange
    let config = KeyStubConfig::default();
    let key = jwk_key(&config.key_id.to_string());
    let auth = Auth::start_with(config, &[]).await;
    redis_delete(&key).await;

    // Act
    auth.register(ALICE, PASSWORD).await.unwrap();

    // Assert
    assert!(redis_has(&key).await);
}

#[tokio::test]
async fn signing_key_already_in_redis_is_not_fetched_again() {
    // Arrange
    let auth = Auth::start().await;
    auth.wait_until_jwks_loaded().await;
    auth.register(ALICE, PASSWORD).await.unwrap();
    let calls_before = auth.key_provider.key_list_calls();

    // Act
    auth.register(BOB, PASSWORD).await.unwrap();

    // Assert
    assert_eq!(auth.key_provider.key_list_calls(), calls_before);
}

#[tokio::test]
async fn key_provider_that_never_answers_returns_unavailable_after_the_timeout() {
    // Arrange
    let auth = Auth::start_with(
        KeyStubConfig {
            sign_hangs: true,
            ..KeyStubConfig::default()
        },
        &[(
            "AUTH__GRPC__KEY_SERVICE_CLIENT__CHANNEL_SETTINGS__TIMEOUT_SECS",
            "1",
        )],
    )
    .await;

    // Act
    let result = auth.register(ALICE, PASSWORD).await;

    // Assert
    assert_eq!(code_of(result), Code::Unavailable);
}
