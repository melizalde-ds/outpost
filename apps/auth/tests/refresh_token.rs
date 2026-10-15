mod common;

use common::auth::{Auth, code_of, millis, redis_expiry, redis_has, session_key};
use pretty_assertions::{assert_eq, assert_ne};
use tonic::Code;

const ALICE: &str = "alice";

#[tokio::test]
async fn valid_token_returns_a_new_refresh_token() {
    // Arrange
    let auth = Auth::start().await;
    let token = auth.sign_up(ALICE).await.refresh_token;

    // Act
    let tokens = auth.refresh(&token).await.unwrap();

    // Assert
    assert_ne!(tokens.refresh_token, token);
}

#[tokio::test]
async fn valid_token_new_session_in_redis_expires_when_the_new_token_does() {
    // Arrange
    let auth = Auth::start().await;
    let token = auth.sign_up(ALICE).await.refresh_token;

    // Act
    let tokens = auth.refresh(&token).await.unwrap();

    // Assert
    assert_eq!(
        redis_expiry(&session_key(&tokens.refresh_token)).await,
        millis(&tokens.refresh_token_expires_at.unwrap())
    );
}

#[tokio::test]
async fn valid_token_is_remembered_until_it_would_have_expired() {
    // Arrange
    let auth = Auth::start().await;
    let token = auth.sign_up(ALICE).await.refresh_token;
    let expiry = redis_expiry(&session_key(&token)).await;

    // Act
    auth.refresh(&token).await.unwrap();

    // Assert
    assert_eq!(redis_expiry(&session_key(&token)).await, expiry);
}

#[tokio::test]
async fn swapped_token_is_refused() {
    // Arrange
    let auth = Auth::start().await;
    let token = auth.sign_up(ALICE).await.refresh_token;
    auth.refresh(&token).await.unwrap();

    // Act
    let result = auth.refresh(&token).await;

    // Assert
    assert_eq!(code_of(result), Code::Unauthenticated);
    assert_eq!(auth.revocation_reasons(ALICE).await, vec!["reuse"]);
}

#[tokio::test]
async fn swapped_token_used_again_ends_the_session() {
    // Arrange
    let auth = Auth::start().await;
    let first = auth.sign_up(ALICE).await.refresh_token;
    let newest = auth.refresh(&first).await.unwrap().refresh_token;
    let _ = auth.refresh(&first).await;

    // Act
    let result = auth.refresh(&newest).await;

    // Assert
    assert_eq!(code_of(result), Code::Unauthenticated);
}

#[tokio::test]
async fn same_token_twice_at_once_works_only_once() {
    // Arrange
    let auth = Auth::start().await;
    let token = auth.sign_up(ALICE).await.refresh_token;

    // Act
    let (first, second) = tokio::join!(auth.refresh(&token), auth.refresh(&token));

    // Assert
    assert_eq!(first.is_ok() as u8 + second.is_ok() as u8, 1);
}

#[tokio::test]
async fn session_past_its_limit_is_refused_even_while_redis_has_the_token() {
    // Arrange
    let auth = Auth::start().await;
    let token = auth.sign_up(ALICE).await.refresh_token;
    auth.pass_session_limit(ALICE).await;

    // Act
    let result = auth.refresh(&token).await;

    // Assert
    assert_eq!(code_of(result), Code::Unauthenticated);
}

#[tokio::test]
async fn switched_off_account_is_refused_and_its_session_ended() {
    // Arrange
    let auth = Auth::start().await;
    let token = auth.sign_up(ALICE).await.refresh_token;
    auth.switch_off(ALICE).await;

    // Act
    let result = auth.refresh(&token).await;

    // Assert
    assert_eq!(code_of(result), Code::Unauthenticated);
    assert!(!redis_has(&session_key(&token)).await);
    assert_eq!(auth.revocation_reasons(ALICE).await, vec!["inactive"]);
}

#[tokio::test]
async fn key_provider_outage_does_not_log_the_user_out() {
    // Arrange
    let auth = Auth::start().await;
    let token = auth.sign_up(ALICE).await.refresh_token;
    auth.key_provider.set_sign_error(Some(Code::Unavailable));
    let _ = auth.refresh(&token).await;
    auth.key_provider.set_sign_error(None);

    // Act
    let result = auth.refresh(&token).await;

    // Assert
    assert!(result.is_ok());
}
