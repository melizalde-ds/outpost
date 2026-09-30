use std::{net::SocketAddr, sync::Arc};

use tonic::transport::Server;
use tracing::info;

use crate::{
    password::argon2::Argon2PasswordHasher,
    proto::auth::v1::auth_service_server::AuthServiceServer,
    repository::postgres::PostgresAuthRepository, service::auth::WeatherAuthService,
};

mod grpc;
mod password;
mod proto;
mod repository;
mod service;
mod settings;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let settings = settings::init()?;

    let auth_repo = PostgresAuthRepository::new().await?;
    info!("Connected to database");

    let passwords = Argon2PasswordHasher::new();
    info!("Auth repository initialized");

    info!("Auth service starting...");
    let auth_service = WeatherAuthService::new(Arc::new(auth_repo), Arc::new(passwords));
    let addr: SocketAddr = settings.grpc.server.get_grpc_address().parse()?;
    info!("Auth service listening on {}", addr);
    Server::builder()
        .add_service(AuthServiceServer::new(auth_service))
        .serve(addr)
        .await?;

    Ok(())
}
