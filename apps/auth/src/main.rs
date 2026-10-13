use std::{net::SocketAddr, sync::Arc, time::Duration};

use redis::Client;
use tonic::transport::{Channel, Endpoint, Server};
use tracing::info;

use crate::{
    password::Argon2PasswordHasher,
    proto::auth::v1::auth_service_server::AuthServiceServer,
    repository::PostgresAuthRepository,
    service::DefaultAuthService,
    settings::{GrpcEndpoint, Settings},
    signing::{KeySigningService, SigningService},
    verification_keys::{JwkRecord, RedisVerificationKeyStore, VerificationKeyStore},
};

mod grpc;
mod password;
mod proto;
mod repository;
mod service;
mod settings;
mod signing;
mod verification_keys;

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

    let client = Client::open(settings.redis.connection_info()?)?;
    let verification_keys = RedisVerificationKeyStore::new(client).await?;

    let signing_channel = create_channel(&settings.grpc.key_service_client)?;

    let auth_repo = PostgresAuthRepository::new(db_pool);
    info!("Auth repository initialized");
    let passwords = Argon2PasswordHasher::new(&settings.password);
    info!("Password hasher initialized");
    let signing_service = Arc::new(KeySigningService::new(signing_channel));
    info!("Signing service initialized");
    let published = publish_verification_keys(&signing_service, &verification_keys).await?;
    info!("Published {} verification keys", published);

    info!("Auth service starting...");
    let auth_service = DefaultAuthService::new(
        Arc::new(auth_repo),
        Arc::new(passwords),
        signing_service,
        Arc::new(verification_keys),
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
    let channel_settings = &endpoint.channel_settings;
    let connect_timeout = Duration::from_secs(channel_settings.connect_timeout);
    let timeout = Duration::from_secs(channel_settings.timeout);
    let channel_endpoint = Endpoint::from_shared(endpoint.client_uri())?
        .connect_timeout(connect_timeout)
        .timeout(timeout);

    Ok(channel_endpoint.connect_lazy())
}

async fn publish_verification_keys(
    client: &KeySigningService,
    verification_keys: &RedisVerificationKeyStore,
) -> anyhow::Result<usize> {
    let keys = client.get_verification_keys().await?;
    let records = keys
        .into_iter()
        .map(JwkRecord::try_from)
        .collect::<Result<Vec<_>, _>>()?;
    verification_keys
        .push_multiple_verification_keys(&records)
        .await?;
    Ok(records.len())
}
