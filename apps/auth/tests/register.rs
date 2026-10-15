mod common;

use common::auth::{
    Auth, PASSWORD, code_of, micros, millis, redis_expiry, session_key, subject_of,
};
use pretty_assertions::assert_eq;
use tonic::Code;

const ALICE: &str = "alice";
const DAY_MICROS: i64 = 24 * 60 * 60 * 1_000_000;

#[tokio::test]
async fn new_username_returns_an_access_token_for_the_new_user() {
    // Arrange
    let auth = Auth::start().await;

    // Act
    let tokens = auth.register(ALICE, PASSWORD).await.unwrap();

    // Assert
    assert_eq!(
        subject_of(&tokens.access_token),
        auth.user_id(ALICE).await.to_string()
    );
}

#[tokio::test]
async fn new_username_stores_an_argon2id_hash_not_the_password() {
    // Arrange
    let auth = Auth::start().await;

    // Act
    auth.register(ALICE, PASSWORD).await.unwrap();

    // Assert
    let hash = auth.password_hash(ALICE).await;
    assert!(hash.starts_with("$argon2id$"));
    assert!(!hash.contains(PASSWORD));
}

#[tokio::test]
async fn new_username_session_in_redis_expires_when_the_refresh_token_does() {
    // Arrange
    let auth = Auth::start().await;

    // Act
    let tokens = auth.register(ALICE, PASSWORD).await.unwrap();

    // Assert
    assert_eq!(
        redis_expiry(&session_key(&tokens.refresh_token)).await,
        millis(&tokens.refresh_token_expires_at.unwrap())
    );
}

#[tokio::test]
async fn new_username_session_ends_23_days_after_the_first_refresh_token_does() {
    // Arrange
    let auth = Auth::start().await;

    // Act
    let tokens = auth.register(ALICE, PASSWORD).await.unwrap();

    // Assert
    let session_limit = auth.session_limit(ALICE).await.timestamp_micros();
    let token_expiry = micros(&tokens.refresh_token_expires_at.unwrap());
    assert_eq!(session_limit - token_expiry, 23 * DAY_MICROS);
}

#[tokio::test]
async fn taken_username_returns_already_exists() {
    // Arrange
    let auth = Auth::start().await;
    auth.sign_up(ALICE).await;

    // Act
    let result = auth.register(ALICE, "another password").await;

    // Assert
    assert_eq!(code_of(result), Code::AlreadyExists);
}

#[tokio::test]
async fn taken_username_keeps_the_first_password() {
    // Arrange
    let auth = Auth::start().await;
    auth.sign_up(ALICE).await;
    let first_hash = auth.password_hash(ALICE).await;

    // Act
    let _ = auth.register(ALICE, "another password").await;

    // Assert
    assert_eq!(auth.password_hash(ALICE).await, first_hash);
}

#[tokio::test]
async fn same_username_twice_at_once_makes_one_account() {
    // Arrange
    let auth = Auth::start().await;

    // Act
    let (first, second) = tokio::join!(
        auth.register(ALICE, PASSWORD),
        auth.register(ALICE, PASSWORD)
    );

    // Assert
    assert_eq!(first.is_ok() as u8 + second.is_ok() as u8, 1);
    assert_eq!(auth.usernames().await, vec![ALICE]);
}

#[tokio::test]
async fn key_provider_down_tells_the_user_to_log_in() {
    // Arrange
    let auth = Auth::start().await;
    auth.key_provider.set_sign_error(Some(Code::Unavailable));

    // Act
    let status = auth.register(ALICE, PASSWORD).await.unwrap_err();

    // Assert
    assert_eq!(status.code(), Code::Unavailable);
    assert_eq!(
        status.message(),
        "account created, but sign-in failed; please log in"
    );
}

#[tokio::test]
async fn key_provider_down_still_lets_the_user_log_in_once_it_is_back() {
    // Arrange
    let auth = Auth::start().await;
    auth.key_provider.set_sign_error(Some(Code::Unavailable));
    let _ = auth.register(ALICE, PASSWORD).await;
    auth.key_provider.set_sign_error(None);

    // Act
    let result = auth.login(ALICE, PASSWORD).await;

    // Assert
    assert!(result.is_ok());
}

#[tokio::test]
async fn empty_username_returns_invalid_argument() {
    // Arrange
    let auth = Auth::start().await;

    // Act
    let result = auth.register("", PASSWORD).await;

    // Assert
    assert_eq!(code_of(result), Code::InvalidArgument);
}

#[tokio::test]
async fn empty_password_returns_invalid_argument() {
    // Arrange
    let auth = Auth::start().await;

    // Act
    let result = auth.register(ALICE, "").await;

    // Assert
    assert_eq!(code_of(result), Code::InvalidArgument);
}

#[tokio::test]
async fn username_differing_only_in_case_returns_already_exists() {
    // Arrange
    let auth = Auth::start().await;
    auth.sign_up(ALICE).await;

    // Act
    let result = auth.register("Alice", PASSWORD).await;

    // Assert
    assert_eq!(code_of(result), Code::AlreadyExists);
}

#[tokio::test]
async fn username_with_surrounding_spaces_returns_already_exists() {
    // Arrange
    let auth = Auth::start().await;
    auth.sign_up(ALICE).await;

    // Act
    let result = auth.register(" alice ", PASSWORD).await;

    // Assert
    assert_eq!(code_of(result), Code::AlreadyExists);
}
