mod common;

use common::auth::{Auth, PASSWORD, code_of, subject_of};
use pretty_assertions::assert_eq;
use tonic::Code;

const ALICE: &str = "alice";
const WRONG_PASSWORD: &str = "wrong horse";

#[tokio::test]
async fn right_password_returns_an_access_token_for_the_user() {
    // Arrange
    let auth = Auth::start().await;
    auth.sign_up(ALICE).await;

    // Act
    let tokens = auth.login(ALICE, PASSWORD).await.unwrap();

    // Assert
    assert_eq!(
        subject_of(&tokens.access_token),
        auth.user_id(ALICE).await.to_string()
    );
}

#[tokio::test]
async fn wrong_password_returns_unauthenticated() {
    // Arrange
    let auth = Auth::start().await;
    auth.sign_up(ALICE).await;

    // Act
    let result = auth.login(ALICE, WRONG_PASSWORD).await;

    // Assert
    assert_eq!(code_of(result), Code::Unauthenticated);
}

#[tokio::test]
async fn username_in_another_case_logs_in_to_the_same_account() {
    // Arrange
    let auth = Auth::start().await;
    auth.sign_up(ALICE).await;

    // Act
    let tokens = auth.login("ALICE", PASSWORD).await.unwrap();

    // Assert
    assert_eq!(
        subject_of(&tokens.access_token),
        auth.user_id(ALICE).await.to_string()
    );
}

#[tokio::test]
async fn username_with_surrounding_spaces_logs_in_to_the_same_account() {
    // Arrange
    let auth = Auth::start().await;
    auth.sign_up(ALICE).await;

    // Act
    let tokens = auth.login(" alice ", PASSWORD).await.unwrap();

    // Assert
    assert_eq!(
        subject_of(&tokens.access_token),
        auth.user_id(ALICE).await.to_string()
    );
}

#[tokio::test]
async fn unknown_username_returns_the_same_error_as_a_wrong_password() {
    // Arrange
    let auth = Auth::start().await;
    auth.sign_up(ALICE).await;
    let wrong_password = auth.login(ALICE, WRONG_PASSWORD).await.unwrap_err();

    // Act
    let unknown_username = auth.login("nobody", PASSWORD).await.unwrap_err();

    // Assert
    assert_eq!(unknown_username.code(), wrong_password.code());
    assert_eq!(unknown_username.message(), wrong_password.message());
}

#[tokio::test]
async fn switched_off_account_returns_unauthenticated() {
    // Arrange
    let auth = Auth::start().await;
    auth.sign_up(ALICE).await;
    auth.switch_off(ALICE).await;

    // Act
    let result = auth.login(ALICE, PASSWORD).await;

    // Assert
    assert_eq!(code_of(result), Code::Unauthenticated);
}
