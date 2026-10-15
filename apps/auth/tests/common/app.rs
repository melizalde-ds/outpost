use std::{net::SocketAddr, process::Stdio, time::Duration};

use tokio::{
    io::{AsyncBufReadExt, AsyncRead, BufReader},
    process::{Child, Command},
};
use tonic::transport::Channel;

use super::{
    APP_PASSWORD, APP_USER, infra, proto::auth::v1::auth_service_client::AuthServiceClient,
};

pub const PEPPER: &str = "test-pepper";

pub struct App {
    _child: Child,
    addr: SocketAddr,
}

impl App {
    pub async fn start(db: &str, key_provider: SocketAddr) -> Self {
        let addr = free_port();
        let mut child = spawn(db, key_provider, addr).await;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                panic!("auth exited before it was ready: {status}");
            }
            if AuthServiceClient::connect(format!("http://{addr}"))
                .await
                .is_ok()
            {
                return Self {
                    _child: child,
                    addr,
                };
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "auth not ready after 30s"
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    pub async fn client(&self) -> AuthServiceClient<Channel> {
        AuthServiceClient::connect(format!("http://{}", self.addr))
            .await
            .unwrap()
    }
}

async fn spawn(db: &str, key_provider: SocketAddr, addr: SocketAddr) -> Child {
    let infra = infra().await;
    let mut child = Command::new(env!("CARGO_BIN_EXE_auth"))
        .env_clear()
        .env("NO_COLOR", "1")
        .env("AUTH__DATABASE__HOST", "127.0.0.1")
        .env("AUTH__DATABASE__PORT", infra.postgres_port.to_string())
        .env("AUTH__DATABASE__USERNAME", APP_USER)
        .env("AUTH__DATABASE__PASSWORD", APP_PASSWORD)
        .env("AUTH__DATABASE__DATABASE_NAME", db)
        .env("AUTH__REDIS__HOST", "127.0.0.1")
        .env("AUTH__REDIS__PORT", infra.redis_port.to_string())
        .env("AUTH__PASSWORD__PEPPER", PEPPER)
        .env("AUTH__GRPC__SERVER__HOST", addr.ip().to_string())
        .env("AUTH__GRPC__SERVER__PORT", addr.port().to_string())
        .env(
            "AUTH__GRPC__KEY_SERVICE_CLIENT__HOST",
            key_provider.ip().to_string(),
        )
        .env(
            "AUTH__GRPC__KEY_SERVICE_CLIENT__PORT",
            key_provider.port().to_string(),
        )
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .expect("start auth");
    forward_output(child.stdout.take().unwrap());
    forward_output(child.stderr.take().unwrap());
    child
}

fn forward_output(stream: impl AsyncRead + Unpin + Send + 'static) {
    tokio::spawn(async move {
        let mut lines = BufReader::new(stream).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            println!("[auth] {line}");
        }
    });
}

fn free_port() -> SocketAddr {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
}
