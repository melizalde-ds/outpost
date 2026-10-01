use std::{net::SocketAddr, sync::Arc, time::Duration};

use tonic::transport::{Channel, Endpoint, Server};
use tracing::info;

use crate::{
    password::argon2::Argon2PasswordHasher,
    proto::auth::v1::auth_service_server::AuthServiceServer,
    repository::postgres::PostgresAuthRepository,
    service::{auth::WeatherAuthService, signing::WeatherSigningService},
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
    let db_pool = settings
        .database
        .pool_options()
        .connect_with(settings.database.connection_options())
        .await?;
    info!("Connected to database");

    let signing_channel = create_channel(&settings.grpc.key_service_client)?;

    let auth_repo = PostgresAuthRepository::new(db_pool);
    info!("Auth repository initialized");
    let passwords = Argon2PasswordHasher::new(&settings.password);
    info!("Password hasher initialized");
    let signing_service = Arc::new(WeatherSigningService::new(signing_channel));
    info!("Signing service initialized");

    info!("Auth service starting...");
    let auth_service =
        WeatherAuthService::new(Arc::new(auth_repo), Arc::new(passwords), signing_service);
    let addr: SocketAddr = settings.grpc.server.get_grpc_address().parse()?;
    info!("Auth service listening on {}", addr);
    Server::builder()
        .add_service(AuthServiceServer::new(auth_service))
        .serve(addr)
        .await?;

    Ok(())
}

fn create_channel(endpoint: &GrpcEndpoint) -> anyhow::Result<Channel> {
    let channel_settings = &endpoint.channel_settings;
    let connect_timeout = Duration::from_secs(channel_settings.connect_timeout);
    let timeout = Duration::from_secs(channel_settings.timeout);
    let channel_endpoint = Endpoint::from_shared(endpoint.client_uri())?
        .connect_timeout(connect_timeout)
        .timeout(timeout);

    Ok(channel_endpoint.connect_lazy())
}
