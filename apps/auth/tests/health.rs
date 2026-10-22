mod common;

use common::{auth::Auth, http};
use hyper::StatusCode;
use pretty_assertions::assert_eq;
use tonic::transport::Channel;
use tonic_health::pb::{
    HealthCheckRequest, health_check_response::ServingStatus, health_client::HealthClient,
};

#[tokio::test]
async fn grpc_health_check_while_running_returns_serving() {
    // Arrange
    let auth = Auth::start().await;
    let channel = Channel::from_shared(format!("http://{}", auth.grpc_addr))
        .unwrap()
        .connect()
        .await
        .unwrap();
    let mut client = HealthClient::new(channel);

    // Act
    let reply = client.check(HealthCheckRequest::default()).await.unwrap();

    // Assert
    assert_eq!(reply.into_inner().status(), ServingStatus::Serving);
}

#[tokio::test]
async fn http_healthz_while_running_returns_200() {
    // Arrange
    let auth = Auth::start().await;

    // Act
    let reply = http::get(auth.http_addr, "/healthz").await;

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
}
