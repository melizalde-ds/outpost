use std::env;
use std::{net::SocketAddr, sync::Arc};

use tonic::transport::Server;
use tracing::info;

use crate::{
    password::argon2::Argon2PasswordHasher,
    proto::auth::v1::auth_service_server::AuthServiceServer,
    repository::postgres::PostgresAuthRepository, service::auth::WeatherAuthService,
};

mod grpc;
mod jwt;
mod password;
mod proto;
mod repository;
mod service;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    info!("Starting WeatherLab Auth Service...");
    let pepper = env::var("AUTH_PEPPER")?;
    let database_url = env::var("AUTH_DATABASE_URL")?;

    let addr: SocketAddr = "[::1]:50051".parse()?;
    let db_pool = sqlx::PgPool::connect(&database_url).await?;
    info!("Connected to database");
    let auth_repo = PostgresAuthRepository::new(db_pool);
    let passwords = Argon2PasswordHasher::new(pepper.into_bytes(), 10);
    info!("Auth repository initialized");

    info!("Auth service starting...");
    let auth_service = WeatherAuthService::new(Arc::new(auth_repo), Arc::new(passwords));

    info!("Auth service listening on {}", addr);
    Server::builder()
        .add_service(AuthServiceServer::new(auth_service))
        .serve(addr)
        .await?;

    Ok(())
}
