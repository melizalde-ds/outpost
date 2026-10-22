use std::{net::SocketAddr, sync::Arc, time::Duration};

use axum::middleware;
use redis::Client;
use sqlx::PgPool;
use tokio::{net::TcpListener, signal};
use tokio_util::sync::CancellationToken;
use tonic::transport::{Channel, Endpoint, Server};
use tonic_health::{ServingStatus, server::health_reporter};
use tower_http::trace::TraceLayer;
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::EnvFilter;

use crate::{
    jwk_cache::RedisJwkCache,
    password::Argon2PasswordHasher,
    proto::auth::v1::auth_service_server::AuthServiceServer,
    refresh_token::RedisRefreshTokenStore,
    repository::{PostgresTokenFamilyRepository, PostgresUserRepository},
    service::DefaultAuthService,
    settings::{
        DatabaseSettings, GrpcEndpoint, GrpcSettings, HttpSettings, RedisSettings, Settings,
    },
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
    let auth_service = Arc::new(build_auth_service(&settings).await?);
    let shutdown = CancellationToken::new();
    let grpc = serve_grpc(auth_service.clone(), &settings.grpc, shutdown.clone());
    let http = serve_http(auth_service.clone(), &settings.http, shutdown.clone());
    let jwks = async {
        auth_service
            .keep_jwks_fresh(settings.http.jwks_refresh(), shutdown.clone())
            .await;
        anyhow::Ok(())
    };
    let on_signal = async {
        shutdown_signal().await?;
        info!("shutdown signal received, finishing in-flight requests");
        shutdown.cancel();
        anyhow::Ok(())
    };

    tokio::try_join!(grpc, http, jwks, on_signal)?;
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
    auth_service: Arc<DefaultAuthService>,
    settings: &GrpcSettings,
    shutdown: CancellationToken,
) -> anyhow::Result<()> {
    let addr: SocketAddr = settings.server.grpc_address().parse()?;
    let (health, health_service) = health_reporter();
    health
        .set_serving::<AuthServiceServer<DefaultAuthService>>()
        .await;
    let stopping = async move {
        shutdown.cancelled().await;
        health
            .set_service_status("", ServingStatus::NotServing)
            .await;
        health
            .set_not_serving::<AuthServiceServer<DefaultAuthService>>()
            .await;
    };
    info!(%addr, "auth service listening");
    Server::builder()
        .timeout(settings.request_timeout())
        .add_service(health_service)
        .add_service(AuthServiceServer::from_arc(auth_service))
        .serve_with_shutdown(addr, stopping)
        .await?;
    info!("auth service stopped");
    Ok(())
}

async fn serve_http(
    auth_service: Arc<DefaultAuthService>,
    settings: &HttpSettings,
    shutdown: CancellationToken,
) -> anyhow::Result<()> {
    let listener = TcpListener::bind(settings.address()).await?;
    info!(addr = %listener.local_addr()?, "HTTP API listening");
    // The trace layer logs no headers, so tokens stay out of the logs.
    let app = http::router(auth_service).layer((
        TraceLayer::new_for_http(),
        middleware::from_fn_with_state(settings.request_timeout(), http::time_limit),
    ));
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown.cancelled_owned())
        .await?;
    info!("HTTP API stopped");
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
