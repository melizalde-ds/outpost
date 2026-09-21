use std::{net::SocketAddr, sync::Arc};

use tonic::transport::Server;
use tracing::info;

use crate::{
    proto::auth::v1::auth_service_server::AuthServiceServer, repository::auth::DbAuthRepository,
    service::auth::WeatherAuthService,
};

mod proto;
mod repository;
mod service;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    info!("Starting WeatherStream Auth Service...");
    let addr: SocketAddr = "[::1]:50051".parse()?;
    let db_pool = sqlx::PgPool::connect("postgres://postgres:password@localhost/auth").await?;
    info!("Connected to database");
    let auth_repo = DbAuthRepository::new(db_pool);
    info!("Auth repository initialized");

    info!("Auth service starting...");
    let auth_service = WeatherAuthService {
        repo: Arc::new(auth_repo),
    };

    info!("Auth service listening on {}", addr);
    Server::builder()
        .add_service(AuthServiceServer::new(auth_service))
        .serve(addr)
        .await?;

    Ok(())
}
