use std::sync::Arc;

use axum::{
    Json,
    extract::State,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde_json::json;

use crate::{http::problem::Problem, service::DefaultAuthService};

const CACHE_CONTROL: &str = "public, max-age=60";
const RETRY_AFTER_SECS: &str = "5";

pub async fn jwks(State(auth): State<Arc<DefaultAuthService>>) -> Response {
    match auth.live_jwks() {
        Some(keys) => (
            [(header::CACHE_CONTROL, CACHE_CONTROL)],
            Json(json!({ "keys": keys })),
        )
            .into_response(),
        None => (
            [(header::RETRY_AFTER, RETRY_AFTER_SECS)],
            Problem::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "public keys are not loaded yet, retry later",
            ),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use chrono::Duration;
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::{
        http::router,
        jwk_cache::{JwkRecord, LocalJwkSet},
        signing::PublicKey,
        test_support::{SIGNING_KEY_ID, TestAuth, get, start_time},
    };

    const JWKS: &str = "/.well-known/jwks.json";

    fn app_with(local_jwks: LocalJwkSet) -> axum::Router {
        let auth = TestAuth::new()
            .with_local_jwks(Arc::new(local_jwks))
            .build();
        router(Arc::new(auth.service))
    }

    fn signing_key() -> JwkRecord {
        JwkRecord::try_from(PublicKey {
            kid: SIGNING_KEY_ID,
            key: "test-public-key".into(),
            exp: start_time() + Duration::days(1),
        })
        .unwrap()
    }

    #[tokio::test]
    async fn jwks_before_first_load_returns_503_with_retry_after() {
        // Arrange
        let app = app_with(LocalJwkSet::default());

        // Act
        let reply = get(app, JWKS).await;

        // Assert
        assert_eq!(reply.status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(reply.headers[header::RETRY_AFTER], RETRY_AFTER_SECS);
        assert_eq!(
            reply.headers[header::CONTENT_TYPE],
            "application/problem+json"
        );
    }

    #[tokio::test]
    async fn jwks_after_load_returns_the_live_keys_with_cache_control() {
        // Arrange
        let local_jwks = LocalJwkSet::default();
        local_jwks.replace(vec![signing_key()]);
        let app = app_with(local_jwks);

        // Act
        let reply = get(app, JWKS).await;

        // Assert
        assert_eq!(reply.status, StatusCode::OK);
        assert_eq!(reply.headers[header::CACHE_CONTROL], CACHE_CONTROL);
        assert_eq!(
            reply.body,
            json!({
                "keys": [{
                    "kty": "OKP",
                    "crv": "Ed25519",
                    "kid": SIGNING_KEY_ID.to_string(),
                    "alg": "EdDSA",
                    "use": "sig",
                    "x": "test-public-key",
                }]
            })
        );
    }
}
