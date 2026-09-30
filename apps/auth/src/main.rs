use std::{net::SocketAddr, sync::Arc};

use secrecy::ExposeSecret;
use sqlx::PgPool;
use tonic::transport::Server;
use tracing::info;

use crate::{
    password::argon2::Argon2PasswordHasher,
    proto::auth::v1::auth_service_server::AuthServiceServer,
    repository::postgres::PostgresAuthRepository, service::auth::WeatherAuthService,
    settings::Settings,
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

    let settings = Settings::load()?;

    let db_pool = PgPool::connect(settings.database.connection_string().expose_secret()).await?;
    info!("Connected to database");

    let auth_repo = PostgresAuthRepository::new(db_pool);
    let passwords = Argon2PasswordHasher::new(&settings.password);
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
