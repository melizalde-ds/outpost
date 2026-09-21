use std::net::SocketAddr;

use crate::repository::auth::DbAuthRepository;

mod proto;
mod repository;
mod service;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _addr: SocketAddr = "[::1]:50051".parse()?;
    let db_pool = sqlx::PgPool::connect("postgres://postgres:password@localhost/auth").await?;
    let _auth_repo = DbAuthRepository::new(db_pool);
    // let auth_service = service::WeatherAuthService {
    //     _repo: Arc::new(auth_repo),
    // };

    // Server::builder()
    //     .add_service(AuthServiceServer::new(auth_service))
    //     .serve(addr)
    //     .await?;

    Ok(())
}
