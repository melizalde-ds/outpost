use std::{net::SocketAddr, sync::Arc, time::Duration};

use redis::Client;
use sqlx::PgPool;
use tokio::signal;
use tokio_util::sync::CancellationToken;
use tonic::transport::{Channel, Endpoint, Server};
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::EnvFilter;

use crate::{
    jwk_cache::RedisJwkCache,
    password::Argon2PasswordHasher,
    proto::auth::v1::auth_service_server::AuthServiceServer,
    refresh_token::RedisRefreshTokenStore,
    repository::{PostgresTokenFamilyRepository, PostgresUserRepository},
    service::DefaultAuthService,
    settings::{DatabaseSettings, GrpcEndpoint, RedisSettings, Settings},
    signing::KeySigningService,
};

mod clock;
mod grpc;
mod http;
mod jwk_cache;
mod password;
mod proto;
mod refresh_token;
mod repository;
mod retry;
mod service;
mod settings;
mod signing;
#[cfg(test)]
mod test_support;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let log_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();
    tracing_subscriber::fmt().with_env_filter(log_filter).init();

    let settings = Settings::load()?;
    let auth_service = build_auth_service(&settings).await?;
    let grpc_shutdown = CancellationToken::new();
    let grpc = serve_grpc(auth_service, &settings.grpc.server, grpc_shutdown.clone());
    let shutdown = async {
        shutdown_signal().await?;
        info!("shutdown signal received, finishing in-flight requests");
        grpc_shutdown.cancel();
        anyhow::Ok(())
    };

    tokio::try_join!(grpc, shutdown)?;
    Ok(())
}

async fn build_auth_service(settings: &Settings) -> anyhow::Result<DefaultAuthService> {
    let db_pool = connect_database(&settings.database).await?;
    let (jwk_cache, refresh_tokens) = connect_redis(&settings.redis).await?;
    let passwords = Argon2PasswordHasher::new(&settings.password)?;
    let signing_channel = create_channel(&settings.grpc.key_service_client)?;

    Ok(DefaultAuthService::new(
        Arc::new(PostgresUserRepository::new(db_pool.clone())),
        Arc::new(PostgresTokenFamilyRepository::new(db_pool)),
        Arc::new(passwords),
        Arc::new(KeySigningService::new(signing_channel)),
        Arc::new(jwk_cache),
        Arc::new(refresh_tokens),
        &settings.refresh_token,
    ))
}

async fn connect_database(settings: &DatabaseSettings) -> anyhow::Result<PgPool> {
    let pool = settings
        .pool_options()
        .connect_with(settings.connection_options())
        .await?;
    info!("connected to Postgres");
    Ok(pool)
}

async fn connect_redis(
    settings: &RedisSettings,
) -> anyhow::Result<(RedisJwkCache, RedisRefreshTokenStore)> {
    let client = Client::open(settings.connection_info()?)?;
    let config = settings.connection_manager_config();
    let jwk_cache = RedisJwkCache::new(client.clone(), config.clone()).await?;
    let refresh_tokens = RedisRefreshTokenStore::new(client, config).await?;
    info!("connected to Redis");
    Ok((jwk_cache, refresh_tokens))
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

async fn serve_grpc(
    auth_service: DefaultAuthService,
    endpoint: &GrpcEndpoint,
    shutdown: CancellationToken,
) -> anyhow::Result<()> {
    let addr: SocketAddr = endpoint.grpc_address().parse()?;
    info!(%addr, "auth service listening");
    Server::builder()
        .add_service(AuthServiceServer::new(auth_service))
        .serve_with_shutdown(addr, shutdown.cancelled_owned())
        .await?;
    info!("auth service stopped");
    Ok(())
}
async fn shutdown_signal() -> anyhow::Result<()> {
    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())?
            .recv()
            .await;
        anyhow::Ok(())
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<anyhow::Result<()>>();

    tokio::select! {
        result = signal::ctrl_c() => Ok(result?),
        result = terminate => result,
    }
}
