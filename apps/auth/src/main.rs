use std::{net::SocketAddr, sync::Arc, time::Duration};

use redis::Client;
use tonic::transport::{Channel, Endpoint, Server};
use tracing::info;

use crate::{
    jwk_cache::{JwkCache, JwkRecord, RedisJwkCache},
    password::Argon2PasswordHasher,
    proto::auth::v1::auth_service_server::AuthServiceServer,
    refresh_token::RedisRefreshTokenStore,
    repository::PostgresAuthRepository,
    service::DefaultAuthService,
    settings::{GrpcEndpoint, Settings},
    signing::{KeySigningService, SigningService},
};

mod grpc;
mod jwk_cache;
mod password;
mod proto;
mod refresh_token;
mod repository;
mod service;
mod settings;
mod signing;

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
    let jwk_cache = RedisJwkCache::new(client.clone()).await?;
    let refresh_tokens = RedisRefreshTokenStore::new(client).await?;

    let signing_channel = create_channel(&settings.grpc.key_service_client)?;

    let auth_repo = PostgresAuthRepository::new(db_pool);
    info!("Auth repository initialized");
    let passwords = Argon2PasswordHasher::new(&settings.password);
    info!("Password hasher initialized");
    let signing_service = Arc::new(KeySigningService::new(signing_channel));
    info!("Signing service initialized");
    let published = warm_jwk_cache(&signing_service, &jwk_cache).await?;
    info!("Published {} verification keys", published);

    info!("Auth service starting...");
    let auth_service = DefaultAuthService::new(
        Arc::new(auth_repo),
        Arc::new(passwords),
        signing_service,
        Arc::new(jwk_cache),
        Arc::new(refresh_tokens),
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

async fn warm_jwk_cache(
    client: &KeySigningService,
    jwk_cache: &RedisJwkCache,
) -> anyhow::Result<usize> {
    let keys = client.get_verification_keys().await?;
    let records = keys
        .into_iter()
        .map(JwkRecord::try_from)
        .collect::<Result<Vec<_>, _>>()?;
    jwk_cache.insert_many(&records).await?;
    Ok(records.len())
}
