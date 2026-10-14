use std::{net::SocketAddr, sync::Arc, time::Duration};

use redis::Client;
use tonic::transport::{Channel, Endpoint, Server};
use tracing::info;

use crate::{
    jwk_cache::{JwkCache, JwkRecord, RedisJwkCache},
    password::Argon2PasswordHasher,
    proto::auth::v1::auth_service_server::AuthServiceServer,
    refresh_token::RedisRefreshTokenStore,
    repository::{PostgresTokenFamilyRepository, PostgresUserRepository},
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
    info!("connected to database");

    let redis_client = Client::open(settings.redis.connection_info()?)?;
    let signing_channel = create_channel(&settings.grpc.key_service_client)?;

    let users = Arc::new(PostgresUserRepository::new(db_pool.clone()));
    info!("user repository initialized");
    let families = Arc::new(PostgresTokenFamilyRepository::new(db_pool));
    info!("token family repository initialized");
    let jwk_cache = Arc::new(RedisJwkCache::new(redis_client.clone()).await?);
    info!("JWK cache initialized");
    let refresh_tokens = Arc::new(RedisRefreshTokenStore::new(redis_client).await?);
    info!("refresh token store initialized");

    let passwords = Arc::new(Argon2PasswordHasher::new(&settings.password));
    info!("password hasher initialized");
    let signer = Arc::new(KeySigningService::new(signing_channel));
    info!("signing service initialized");

    let cached = warm_jwk_cache(&signer, &jwk_cache).await?;
    info!(keys = cached, "JWK cache warmed");

    let auth_service = DefaultAuthService::new(
        users,
        families,
        passwords,
        signer,
        jwk_cache,
        refresh_tokens,
        &settings.refresh_token,
    );
    let addr: SocketAddr = settings.grpc.server.grpc_address().parse()?;
    info!(%addr, "auth service listening");
    Server::builder()
        .add_service(AuthServiceServer::new(auth_service))
        .serve(addr)
        .await?;

    Ok(())
}

fn create_channel(endpoint: &GrpcEndpoint) -> anyhow::Result<Channel> {
    let channel_settings = &endpoint.channel_settings;
    let connect_timeout = Duration::from_secs(channel_settings.connect_timeout_secs);
    let timeout = Duration::from_secs(channel_settings.timeout_secs);
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
