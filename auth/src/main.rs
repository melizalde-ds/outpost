use std::env;
use std::{net::SocketAddr, sync::Arc};

use tonic::transport::Server;
use tracing::info;

use crate::password::engine::PasswordEngine;
use crate::{
    proto::auth::v1::auth_service_server::AuthServiceServer, repository::auth::DbAuthRepository,
    service::auth::WeatherAuthService,
};

mod password;
mod proto;
mod repository;
mod service;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    info!("Starting WeatherStream Auth Service...");
    // Insecure - for demonstration purposes only. In production, use Hashicorp Vault or AWS Secrets Manager.
    let append = env::var("AUTH_APPEND")?;

    let addr: SocketAddr = "[::1]:50051".parse()?;
    let db_pool = sqlx::PgPool::connect("postgres://postgres:password@localhost/auth").await?;
    info!("Connected to database");
    let auth_repo = DbAuthRepository::new(db_pool);
    let password_engine = PasswordEngine::new(append.into_bytes(), 10);
    info!("Auth repository initialized");

    info!("Auth service starting...");
    let auth_service = WeatherAuthService::new(Arc::new(auth_repo), Arc::new(password_engine));

    info!("Auth service listening on {}", addr);
    Server::builder()
        .add_service(AuthServiceServer::new(auth_service))
        .serve(addr)
        .await?;

    Ok(())
}
