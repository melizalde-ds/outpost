mod common;

use common::{
    admin,
    app::App,
    fresh_db,
    key_stub::{KeyStub, KeyStubConfig},
    proto::auth::v1::RegisterRequest,
};
use pretty_assertions::assert_eq;
use serde::Deserialize;
use uuid::Uuid;

const ALICE: &str = "alice";
const PASSWORD: &str = "correct horse";

#[derive(Deserialize)]
struct Claims {
    sub: String,
}

fn register_request(username: &str, password: &str) -> RegisterRequest {
    RegisterRequest {
        username: username.into(),
        password: password.into(),
    }
}

#[tokio::test]
async fn new_username_returns_an_access_token_for_the_new_user() {
    // Arrange
    let db = fresh_db().await;
    let key_provider = KeyStub::start(KeyStubConfig::default()).await;
    let app = App::start(&db, key_provider.addr).await;
    let mut client = app.client().await;

    // Act
    let response = client
        .register(register_request(ALICE, PASSWORD))
        .await
        .unwrap();

    // Assert
    let access_token = response.into_inner().tokens.unwrap().access_token;
    let claims = jsonwebtoken::dangerous::insecure_decode::<Claims>(&access_token)
        .unwrap()
        .claims;
    let user_id: Uuid = sqlx::query_scalar("SELECT user_id FROM auth WHERE username = $1")
        .bind(ALICE)
        .fetch_one(&admin(&db).await)
        .await
        .unwrap();
    assert_eq!(claims.sub, user_id.to_string());
}
