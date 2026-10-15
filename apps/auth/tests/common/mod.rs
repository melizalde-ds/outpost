#![allow(dead_code)]

use std::{path::Path, time::Duration};

use sqlx::{Connection, Executor, PgConnection, PgPool, postgres::PgPoolOptions};
use testcontainers_modules::{
    postgres::Postgres,
    redis::Redis,
    testcontainers::{ContainerAsync, ImageExt, runners::AsyncRunner},
};
use tokio::sync::OnceCell;

pub mod app;
pub mod key_stub;

#[path = "../../src/proto.rs"]
pub mod proto;

const SUPERUSER: &str = "postgres";
const SUPERUSER_PASSWORD: &str = "postgres";
pub const APP_USER: &str = "auth_app";
pub const APP_PASSWORD: &str = "app-password";
const MIGRATOR_PASSWORD: &str = "migrator-password";
const TEMPLATE_DB: &str = "auth";
const LABEL: (&str, &str) = ("outpost.tests", "auth");

pub struct Infra {
    _postgres: ContainerAsync<Postgres>,
    _redis: ContainerAsync<Redis>,
    pub postgres_port: u16,
    pub redis_port: u16,
}

static INFRA: OnceCell<Infra> = OnceCell::const_new();

pub async fn infra() -> &'static Infra {
    INFRA.get_or_init(start_infra).await
}

async fn start_infra() -> Infra {
    let users_script = Path::new(env!("CARGO_MANIFEST_DIR")).join("db/init/00-users.sh");
    let postgres = Postgres::default()
        .with_db_name(TEMPLATE_DB)
        .with_user(SUPERUSER)
        .with_password(SUPERUSER_PASSWORD)
        .with_tag("18")
        .with_env_var("MIGRATION_PASSWORD", MIGRATOR_PASSWORD)
        .with_env_var("APP_PASSWORD", APP_PASSWORD)
        .with_copy_to("/docker-entrypoint-initdb.d/00-users.sh", users_script)
        .with_label(LABEL.0, LABEL.1)
        .start()
        .await
        .expect("start postgres");
    let redis = Redis::default()
        .with_tag("8")
        .with_label(LABEL.0, LABEL.1)
        .start()
        .await
        .expect("start redis");
    let postgres_port = postgres.get_host_port_ipv4(5432).await.unwrap();
    let redis_port = redis.get_host_port_ipv4(6379).await.unwrap();

    let mut migrator = connect_with_retry(&url(
        "auth_migrator",
        MIGRATOR_PASSWORD,
        postgres_port,
        TEMPLATE_DB,
    ))
    .await;
    sqlx::migrate!()
        .run(&mut migrator)
        .await
        .expect("run migrations");
    migrator.close().await.unwrap();

    Infra {
        _postgres: postgres,
        _redis: redis,
        postgres_port,
        redis_port,
    }
}

async fn connect_with_retry(url: &str) -> PgConnection {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    loop {
        match PgConnection::connect(url).await {
            Ok(conn) => return conn,
            Err(e) if tokio::time::Instant::now() > deadline => panic!("postgres not ready: {e}"),
            Err(_) => tokio::time::sleep(Duration::from_millis(100)).await,
        }
    }
}

fn url(user: &str, password: &str, port: u16, db: &str) -> String {
    format!("postgres://{user}:{password}@127.0.0.1:{port}/{db}")
}

pub async fn fresh_db() -> String {
    let infra = infra().await;
    let name = format!("t_{}", uuid::Uuid::now_v7().simple());
    let mut admin = PgConnection::connect(&url(
        SUPERUSER,
        SUPERUSER_PASSWORD,
        infra.postgres_port,
        "postgres",
    ))
    .await
    .unwrap();
    let create = format!(r#"CREATE DATABASE "{name}" TEMPLATE "{TEMPLATE_DB}""#);
    admin
        .execute(sqlx::AssertSqlSafe(create))
        .await
        .expect("create test database");
    name
}

pub async fn admin(db: &str) -> PgPool {
    let infra = infra().await;
    PgPoolOptions::new()
        .max_connections(1)
        .connect(&url(SUPERUSER, SUPERUSER_PASSWORD, infra.postgres_port, db))
        .await
        .unwrap()
}

pub async fn redis() -> redis::aio::MultiplexedConnection {
    let infra = infra().await;
    redis::Client::open(format!("redis://127.0.0.1:{}", infra.redis_port))
        .unwrap()
        .get_multiplexed_async_connection()
        .await
        .unwrap()
}
