use std::sync::Arc;

use tonic::transport::Server;

use crate::proto::auth::{self, v1::auth_service_server::AuthServiceServer};

mod proto;
mod service;
mod user;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let addr = "[::1]:50051".parse()?;
    let db_pool = sqlx::PgPool::connect("postgres://postgres:password@localhost/auth").await?;
    let auth_repo = user::DbAuthRepository::new(db_pool);
    let auth_service = service::WeatherAuthService {
        _repo: Arc::new(auth_repo),
    };

    Server::builder()
        .add_service(AuthServiceServer::new(auth_service))
        .serve(addr)
        .await?;

    Ok(())
}
