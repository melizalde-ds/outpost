mod common;

use std::time::{Duration, Instant};

use common::{
    LABEL,
    auth::{Auth, PASSWORD, code_of},
    key_stub::KeyStubConfig,
    wait_for_redis,
};
use testcontainers_modules::{
    redis::Redis,
    testcontainers::{ContainerAsync, ImageExt, core::IntoContainerPort, runners::AsyncRunner},
};
use tonic::Code;

const ALICE: &str = "alice";

struct OwnRedis {
    container: ContainerAsync<Redis>,
    port: u16,
}

impl OwnRedis {
    async fn start() -> Self {
        let port = std::net::TcpListener::bind(("127.0.0.1", 0))
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let container = Redis::default()
            .with_tag("8")
            .with_label(LABEL.0, LABEL.1)
            .with_mapped_port(port, 6379.tcp())
            .start()
            .await
            .expect("start redis");
        wait_for_redis(port).await;
        Self { container, port }
    }

    async fn crash(&self) {
        self.container.stop_with_timeout(Some(0)).await.unwrap();
    }

    async fn restart(&self) {
        self.container.start().await.unwrap();
        wait_for_redis(self.port).await;
    }

    async fn auth(&self) -> Auth {
        let port = self.port.to_string();
        Auth::start_with(KeyStubConfig::default(), &[("AUTH__REDIS__PORT", &port)]).await
    }
}

#[tokio::test]
async fn redis_down_login_returns_unavailable() {
    // Arrange
    let redis = OwnRedis::start().await;
    let auth = redis.auth().await;
    auth.sign_up(ALICE).await;
    redis.crash().await;

    // Act
    let result = auth.login(ALICE, PASSWORD).await;

    // Assert
    assert_eq!(code_of(result), Code::Unavailable);
}

#[tokio::test]
async fn redis_down_login_answers_within_3_seconds() {
    // Arrange
    let redis = OwnRedis::start().await;
    let auth = redis.auth().await;
    auth.sign_up(ALICE).await;
    redis.crash().await;
    let started = Instant::now();

    // Act
    let _ = auth.login(ALICE, PASSWORD).await;

    // Assert
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "{:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn redis_back_after_a_crash_login_works_again_without_restarting_auth() {
    // Arrange
    let redis = OwnRedis::start().await;
    let auth = redis.auth().await;
    auth.sign_up(ALICE).await;
    redis.crash().await;
    let _ = auth.login(ALICE, PASSWORD).await;
    redis.restart().await;

    // Act
    let result = auth.login(ALICE, PASSWORD).await;

    // Assert
    assert!(result.is_ok(), "{result:?}");
}
