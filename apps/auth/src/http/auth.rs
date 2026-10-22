use std::sync::Arc;

use axum::{
    Json,
    extract::{State, rejection::JsonRejection},
    http::StatusCode,
};
use secrecy::{ExposeSecret, SecretSlice, SecretString};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{http::problem::Problem, service::DefaultAuthService};

type Auth = State<Arc<DefaultAuthService>>;
type Body<T> = Result<Json<T>, JsonRejection>;

#[derive(Debug, Deserialize)]
pub struct CredentialsBody {
    username: String,
    password: SecretString,
}

#[derive(Debug, Deserialize)]
pub struct RefreshTokenBody {
    refresh_token: SecretString,
}

fn password_bytes(password: &SecretString) -> SecretSlice<u8> {
    SecretSlice::from(password.expose_secret().as_bytes().to_vec())
}

pub async fn register(
    State(auth): Auth,
    body: Body<CredentialsBody>,
) -> Result<Json<Value>, Problem> {
    let Json(body) = body?;
    let tokens = auth
        .register(body.username, password_bytes(&body.password))
        .await?;
    Ok(Json(json!({ "tokens": tokens })))
}

pub async fn login(State(auth): Auth, body: Body<CredentialsBody>) -> Result<Json<Value>, Problem> {
    let Json(body) = body?;
    let tokens = auth
        .login(body.username, password_bytes(&body.password))
        .await?;
    Ok(Json(json!({ "tokens": tokens })))
}

pub async fn refresh(
    State(auth): Auth,
    body: Body<RefreshTokenBody>,
) -> Result<Json<Value>, Problem> {
    let Json(body) = body?;
    let tokens = auth.refresh(body.refresh_token.expose_secret()).await?;
    Ok(Json(json!({ "tokens": tokens })))
}

pub async fn logout(
    State(auth): Auth,
    body: Body<RefreshTokenBody>,
) -> Result<StatusCode, Problem> {
    let Json(body) = body?;
    auth.logout(body.refresh_token.expose_secret()).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn logout_all(
    State(auth): Auth,
    body: Body<RefreshTokenBody>,
) -> Result<StatusCode, Problem> {
    let Json(body) = body?;
    auth.logout_all(body.refresh_token.expose_secret()).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use axum::{Router, http::header};
    use pretty_assertions::{assert_eq, assert_ne};
    use rstest::rstest;
    use tonic::Status;
    use uuid::Uuid;

    use super::*;
    use crate::{
        http::router,
        repository::UserId,
        signing::{MockSigningService, SigningError},
        test_support::{FakeSigner, HttpReply, TestAuth, post, post_json},
    };

    const ALICE: &str = "alice";
    const PASSWORD: &str = "correct horse";
    const WRONG_PASSWORD: &str = "wrong horse";
    const GARBAGE_TOKEN: &str = "not-a-refresh-token";
    const PROBLEM_JSON: &str = "application/problem+json";

    fn app() -> Router {
        router(Arc::new(TestAuth::new().build().service))
    }

    fn credentials(username: &str, password: &str) -> Value {
        json!({ "username": username, "password": password })
    }

    fn refresh_body(refresh_token: &str) -> Value {
        json!({ "refresh_token": refresh_token })
    }

    async fn sign_up(app: &Router) -> String {
        let reply = post_json(
            app.clone(),
            "/v1/auth/register",
            credentials(ALICE, PASSWORD),
        )
        .await;
        refresh_token_of(&reply)
    }

    fn refresh_token_of(reply: &HttpReply) -> String {
        reply.body["tokens"]["refresh_token"]
            .as_str()
            .unwrap()
            .to_owned()
    }

    #[tokio::test]
    async fn register_new_user_returns_200_with_tokens() {
        // Arrange
        let app = app();

        // Act
        let reply = post_json(app, "/v1/auth/register", credentials(ALICE, PASSWORD)).await;

        // Assert
        assert_eq!(reply.status, StatusCode::OK);
        let tokens = &reply.body["tokens"];
        assert_eq!(
            tokens["access_token"],
            FakeSigner::token_for(UserId(Uuid::from_u128(1)))
        );
        assert!(tokens["refresh_token"].is_string());
        assert_eq!(tokens["refresh_token_expires_at"], "2026-01-08T00:00:00Z");
    }

    #[tokio::test]
    async fn login_registered_user_returns_200_with_tokens() {
        // Arrange
        let app = app();
        sign_up(&app).await;

        // Act
        let reply = post_json(app, "/v1/auth/login", credentials(ALICE, PASSWORD)).await;

        // Assert
        assert_eq!(reply.status, StatusCode::OK);
        assert!(reply.body["tokens"]["access_token"].is_string());
    }

    #[tokio::test]
    async fn login_wrong_password_returns_401_problem() {
        // Arrange
        let app = app();
        sign_up(&app).await;

        // Act
        let reply = post_json(app, "/v1/auth/login", credentials(ALICE, WRONG_PASSWORD)).await;

        // Assert
        assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
        assert_eq!(reply.headers[header::CONTENT_TYPE], PROBLEM_JSON);
        assert_eq!(
            reply.body,
            json!({
                "type": "about:blank",
                "title": "Unauthorized",
                "status": 401,
                "detail": "invalid credentials",
            })
        );
    }

    #[tokio::test]
    async fn register_taken_username_returns_409() {
        // Arrange
        let app = app();
        sign_up(&app).await;

        // Act
        let reply = post_json(app, "/v1/auth/register", credentials(ALICE, PASSWORD)).await;

        // Assert
        assert_eq!(reply.status, StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn register_empty_password_returns_400() {
        // Arrange
        let app = app();

        // Act
        let reply = post_json(app, "/v1/auth/register", credentials(ALICE, "")).await;

        // Assert
        assert_eq!(reply.status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn register_key_provider_down_returns_503() {
        // Arrange
        let mut signer = MockSigningService::new();
        signer
            .expect_sign()
            .returning(|_| Err(SigningError::Call(Status::unavailable("down"))));
        let app = router(Arc::new(
            TestAuth::new().with_signer(signer).build().service,
        ));

        // Act
        let reply = post_json(app, "/v1/auth/register", credentials(ALICE, PASSWORD)).await;

        // Assert
        assert_eq!(reply.status, StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn refresh_valid_token_returns_200_with_a_new_refresh_token() {
        // Arrange
        let app = app();
        let token = sign_up(&app).await;

        // Act
        let reply = post_json(app, "/v1/auth/refresh", refresh_body(&token)).await;

        // Assert
        assert_eq!(reply.status, StatusCode::OK);
        assert_ne!(refresh_token_of(&reply), token);
    }

    #[rstest]
    #[case::refresh("/v1/auth/refresh")]
    #[case::logout_all("/v1/auth/logout-all")]
    #[tokio::test]
    async fn garbage_token_returns_401(#[case] uri: &str) {
        // Arrange
        let app = app();

        // Act
        let reply = post_json(app, uri, refresh_body(GARBAGE_TOKEN)).await;

        // Assert
        assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
    }

    #[rstest]
    #[case::logout("/v1/auth/logout")]
    #[case::logout_all("/v1/auth/logout-all")]
    #[tokio::test]
    async fn logout_valid_token_returns_204(#[case] uri: &str) {
        // Arrange
        let app = app();
        let token = sign_up(&app).await;

        // Act
        let reply = post_json(app, uri, refresh_body(&token)).await;

        // Assert
        assert_eq!(reply.status, StatusCode::NO_CONTENT);
    }

    #[rstest]
    #[case::logout("/v1/auth/logout")]
    #[case::logout_all("/v1/auth/logout-all")]
    #[tokio::test]
    async fn refresh_after_logout_returns_401(#[case] logout_uri: &str) {
        // Arrange
        let app = app();
        let token = sign_up(&app).await;
        post_json(app.clone(), logout_uri, refresh_body(&token)).await;

        // Act
        let reply = post_json(app, "/v1/auth/refresh", refresh_body(&token)).await;

        // Assert
        assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
    }

    #[rstest]
    #[case::not_json(Some("application/json"), "{", StatusCode::BAD_REQUEST)]
    #[case::no_content_type(
        None,
        r#"{"username":"alice","password":"x"}"#,
        StatusCode::UNSUPPORTED_MEDIA_TYPE
    )]
    #[case::missing_field(
        Some("application/json"),
        r#"{"username":"alice"}"#,
        StatusCode::UNPROCESSABLE_ENTITY
    )]
    #[case::wrong_type(
        Some("application/json"),
        r#"{"username":"alice","password":1}"#,
        StatusCode::UNPROCESSABLE_ENTITY
    )]
    #[tokio::test]
    async fn login_bad_body_returns_problem(
        #[case] content_type: Option<&str>,
        #[case] body: &str,
        #[case] expected: StatusCode,
    ) {
        // Arrange
        let app = app();

        // Act
        let reply = post(app, "/v1/auth/login", content_type, body).await;

        // Assert
        assert_eq!(reply.status, expected);
        assert_eq!(reply.headers[header::CONTENT_TYPE], PROBLEM_JSON);
    }

    #[test]
    fn debug_credentials_body_does_not_leak_the_password() {
        // Arrange
        let body: CredentialsBody = serde_json::from_value(credentials(ALICE, PASSWORD)).unwrap();

        // Act
        let printed = format!("{body:?}");

        // Assert
        assert!(!printed.contains(PASSWORD));
    }

    #[test]
    fn debug_refresh_token_body_does_not_leak_the_token() {
        // Arrange
        let body: RefreshTokenBody = serde_json::from_value(refresh_body(GARBAGE_TOKEN)).unwrap();

        // Act
        let printed = format!("{body:?}");

        // Assert
        assert!(!printed.contains(GARBAGE_TOKEN));
    }
}
