use std::{sync::Arc, time::Duration};

use axum::{
    Router,
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use tracing::warn;

use crate::{http::problem::Problem, service::DefaultAuthService};

mod auth;
mod jwks;
mod problem;

pub fn router(auth: Arc<DefaultAuthService>) -> Router {
    Router::new()
        .route("/v1/auth/register", post(auth::register))
        .route("/v1/auth/login", post(auth::login))
        .route("/v1/auth/refresh", post(auth::refresh))
        .route("/v1/auth/logout", post(auth::logout))
        .route("/v1/auth/logout-all", post(auth::logout_all))
        .route("/.well-known/jwks.json", get(jwks::jwks))
        .with_state(auth)
}

pub async fn time_limit(State(limit): State<Duration>, request: Request, next: Next) -> Response {
    let path = request.uri().path().to_owned();
    match tokio::time::timeout(limit, next.run(request)).await {
        Ok(response) => response,
        Err(_) => {
            warn!(%path, "HTTP request timed out");
            Problem::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "service unavailable, retry later",
            )
            .into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use axum::{http::header, middleware};
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::test_support::get as get_path;

    const LIMIT: Duration = Duration::from_secs(10);

    fn app_taking(duration: Duration) -> Router {
        Router::new()
            .route("/", get(move || tokio::time::sleep(duration)))
            .layer(middleware::from_fn_with_state(LIMIT, time_limit))
    }

    #[tokio::test(start_paused = true)]
    async fn time_limit_request_past_the_limit_returns_503_problem() {
        // Arrange
        let app = app_taking(LIMIT + Duration::from_secs(1));

        // Act
        let reply = get_path(app, "/").await;

        // Assert
        assert_eq!(reply.status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            reply.headers[header::CONTENT_TYPE],
            "application/problem+json"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn time_limit_request_within_the_limit_passes_through() {
        // Arrange
        let app = app_taking(LIMIT - Duration::from_secs(1));

        // Act
        let reply = get_path(app, "/").await;

        // Assert
        assert_eq!(reply.status, StatusCode::OK);
    }
}
