use std::sync::Arc;

use axum::{
    Router,
    routing::{get, post},
};

use crate::service::DefaultAuthService;

mod auth;
mod jwks;
mod problem;

#[cfg_attr(not(test), expect(dead_code, reason = "served from main"))]
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
