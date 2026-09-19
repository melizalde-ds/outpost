use tonic::transport::Server;

use crate::proto::auth::v1::auth_service_server::AuthServiceServer;

mod proto;
mod service;
mod user;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let addr = "[::1]:50051".parse()?;
    let auth_service = service::WeatherAuthService;

    Server::builder()
        .add_service(AuthServiceServer::new(auth_service))
        .serve(addr)
        .await?;

    Ok(())
}
