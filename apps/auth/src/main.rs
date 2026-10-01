use std::{net::SocketAddr, sync::Arc};

use secrecy::ExposeSecret;
use sqlx::PgPool;
use tonic::transport::{Channel, Endpoint, Server, channel};
use tracing::info;

use crate::{
    password::argon2::Argon2PasswordHasher,
    proto::{auth::v1::auth_service_server::AuthServiceServer, key},
    repository::postgres::PostgresAuthRepository,
    service::{
        auth::WeatherAuthService,
        signing::{self, WeatherSigningService},
    },
    settings::{Settings, grpc::GrpcEndpoint},
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

    let signing_channel = create_channel(&settings.grpc.key_service_client)?;

    let auth_repo = PostgresAuthRepository::new(db_pool);
    let passwords = Argon2PasswordHasher::new(&settings.password);
    let signing_service = WeatherSigningService::new(signing_channel);
    info!("Auth repository initialized");
    info!("Signing service initialized");

    info!("Auth service starting...");
    let auth_service = WeatherAuthService::new(
        Arc::new(auth_repo),
        Arc::new(passwords),
        Arc::new(signing_service),
    );
    let addr: SocketAddr = settings.grpc.server.get_grpc_address().parse()?;
    info!("Auth service listening on {}", addr);
    Server::builder()
        .add_service(AuthServiceServer::new(auth_service))
        .serve(addr)
        .await?;

    Ok(())
}

fn create_channel(endpoint: &GrpcEndpoint) -> anyhow::Result<Channel> {
    let mut channel_endpoint = Endpoint::from_shared(endpoint.get_grpc_address())?;

    if let Some(channel_settings) = &endpoint.channel_settings {
        let connect_timeout = std::time::Duration::from_secs(channel_settings.connect_timeout);
        let timeout = std::time::Duration::from_secs(channel_settings.timeout);
        channel_endpoint = channel_endpoint
            .connect_timeout(connect_timeout)
            .timeout(timeout);
    }

    Ok(channel_endpoint.connect_lazy())
}
