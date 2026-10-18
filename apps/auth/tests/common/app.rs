use std::{
    net::{Ipv4Addr, SocketAddr},
    process::{ExitStatus, Stdio},
    time::Duration,
};

use tokio::{
    io::{AsyncBufReadExt, AsyncRead, BufReader},
    process::{Child, Command},
};
use tonic::transport::Channel;
use uuid::Uuid;

use super::{
    APP_PASSWORD, APP_USER, infra,
    proto::auth::v1::{LogoutRequest, auth_service_client::AuthServiceClient},
};

pub const PEPPER: &str = "test-pepper";

pub struct App {
    _child: Child,
    client: AuthServiceClient<Channel>,
}

impl App {
    pub async fn start(db: &str, key_provider: SocketAddr) -> Self {
        Self::start_with(db, key_provider, &[]).await
    }

    pub async fn start_with(db: &str, key_provider: SocketAddr, settings: &[(&str, &str)]) -> Self {
        let addr = free_address();
        let mut child = spawn(db, key_provider, addr, settings).await;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                panic!("auth exited before it was ready: {status}");
            }
            if let Some(client) = answering_client(addr).await {
                return Self {
                    _child: child,
                    client,
                };
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "auth not ready after 30s"
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    pub async fn exit_status(db: &str, key_provider: SocketAddr) -> ExitStatus {
        let mut child = spawn(db, key_provider, free_address(), &[]).await;
        tokio::time::timeout(Duration::from_secs(30), child.wait())
            .await
            .expect("auth still running after 30s")
            .unwrap()
    }

    pub fn client(&self) -> AuthServiceClient<Channel> {
        self.client.clone()
    }
}

// A connection alone can be a false positive, so require a real answer: auth accepts any logout.
async fn answering_client(addr: SocketAddr) -> Option<AuthServiceClient<Channel>> {
    let mut client = AuthServiceClient::connect(format!("http://{addr}"))
        .await
        .ok()?;
    client.logout(LogoutRequest::default()).await.ok()?;
    Some(client)
}

async fn spawn(
    db: &str,
    key_provider: SocketAddr,
    addr: SocketAddr,
    settings: &[(&str, &str)],
) -> Child {
    let infra = infra().await;
    let mut child = Command::new(env!("CARGO_BIN_EXE_auth"))
        .env_clear()
        .envs(std::env::var_os("SYSTEMROOT").map(|root| ("SYSTEMROOT", root)))
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
        .envs(settings.iter().copied())
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

// Each auth gets its own loopback address, so a port picked here can't be taken by another test's server.
pub fn free_address() -> SocketAddr {
    let [a, b, c, ..] = *Uuid::new_v4().as_bytes();
    std::net::TcpListener::bind((Ipv4Addr::new(127, a, b, c.max(1)), 0))
        .unwrap()
        .local_addr()
        .unwrap()
}
