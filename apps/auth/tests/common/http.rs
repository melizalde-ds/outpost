use std::net::SocketAddr;

use http_body_util::{BodyExt, Full};
use hyper::{
    HeaderMap, Method, Request, StatusCode,
    body::Bytes,
    header::{CONTENT_TYPE, HOST},
};
use hyper_util::rt::TokioIo;
use serde_json::Value;
use tokio::net::TcpStream;

pub struct HttpReply {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Value,
}

pub async fn get(addr: SocketAddr, path: &str) -> HttpReply {
    send(addr, Method::GET, path, Bytes::new()).await
}

pub async fn post(addr: SocketAddr, path: &str, body: Value) -> HttpReply {
    send(addr, Method::POST, path, Bytes::from(body.to_string())).await
}

async fn send(addr: SocketAddr, method: Method, path: &str, body: Bytes) -> HttpReply {
    let stream = TcpStream::connect(addr).await.unwrap();
    let (mut sender, connection) = hyper::client::conn::http1::handshake(TokioIo::new(stream))
        .await
        .unwrap();
    tokio::spawn(connection);
    let request = Request::builder()
        .method(method)
        .uri(path)
        .header(HOST, addr.to_string())
        .header(CONTENT_TYPE, "application/json")
        .body(Full::new(body))
        .unwrap();
    let response = sender.send_request(request).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    HttpReply {
        status,
        headers,
        body: serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    }
}
