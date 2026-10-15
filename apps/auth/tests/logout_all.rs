mod common;

use common::auth::{Auth, code_of};
use pretty_assertions::assert_eq;
use tonic::Code;

const ALICE: &str = "alice";
const BOB: &str = "bob";

#[tokio::test]
async fn ends_every_session_of_the_user() {
    // Arrange
    let auth = Auth::start().await;
    let phone = auth.sign_up(ALICE).await.refresh_token;
    let laptop = auth.sign_in(ALICE).await.refresh_token;
    auth.logout_all(&phone).await.unwrap();

    // Act
    let result = auth.refresh(&laptop).await;

    // Assert
    assert_eq!(code_of(result), Code::Unauthenticated);
}

#[tokio::test]
async fn records_why_each_session_ended() {
    // Arrange
    let auth = Auth::start().await;
    let phone = auth.sign_up(ALICE).await.refresh_token;
    auth.sign_in(ALICE).await;

    // Act
    auth.logout_all(&phone).await.unwrap();

    // Assert
    assert_eq!(
        auth.revocation_reasons(ALICE).await,
        vec!["logout_all", "logout_all"]
    );
}

#[tokio::test]
async fn other_users_sessions_keep_working() {
    // Arrange
    let auth = Auth::start().await;
    let alice = auth.sign_up(ALICE).await.refresh_token;
    let bob = auth.sign_up(BOB).await.refresh_token;
    auth.logout_all(&alice).await.unwrap();

    // Act
    let result = auth.refresh(&bob).await;

    // Assert
    assert!(result.is_ok());
}
