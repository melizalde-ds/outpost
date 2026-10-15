mod common;

use common::auth::{Auth, code_of};
use pretty_assertions::assert_eq;
use tonic::Code;

const ALICE: &str = "alice";

#[tokio::test]
async fn logged_out_token_cannot_refresh() {
    // Arrange
    let auth = Auth::start().await;
    let token = auth.sign_up(ALICE).await.refresh_token;
    auth.logout(&token).await.unwrap();

    // Act
    let result = auth.refresh(&token).await;

    // Assert
    assert_eq!(code_of(result), Code::Unauthenticated);
    assert_eq!(auth.revocation_reasons(ALICE).await, vec!["logout"]);
}

#[tokio::test]
async fn other_sessions_of_the_user_keep_working() {
    // Arrange
    let auth = Auth::start().await;
    let phone = auth.sign_up(ALICE).await.refresh_token;
    let laptop = auth.sign_in(ALICE).await.refresh_token;
    auth.logout(&phone).await.unwrap();

    // Act
    let result = auth.refresh(&laptop).await;

    // Assert
    assert!(result.is_ok());
}
