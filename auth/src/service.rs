use std::sync::Arc;

use crate::proto::auth::v1::auth_service_server::AuthService;
use crate::proto::auth::v1::{
    LoginRequest, LoginResponse, RefreshTokenRequest, RefreshTokenResponse, RegisterRequest,
    RegisterResponse,
};
use crate::repository::auth::AuthRepository;
use tonic::{Request, Response, Status};

#[derive(Debug)]
pub struct WeatherAuthService {
    pub _repo: Arc<dyn AuthRepository>,
}

#[tonic::async_trait]
impl AuthService for WeatherAuthService {
    async fn register(
        &self,
        _request: Request<RegisterRequest>,
    ) -> Result<Response<RegisterResponse>, Status> {
        Ok(Response::new(RegisterResponse {
            ..Default::default()
        }))
    }

    async fn login(
        &self,
        _request: Request<LoginRequest>,
    ) -> Result<Response<LoginResponse>, Status> {
        Ok(Response::new(LoginResponse {
            ..Default::default()
        }))
    }

    async fn refresh_token(
        &self,
        _request: Request<RefreshTokenRequest>,
    ) -> Result<Response<RefreshTokenResponse>, Status> {
        Ok(Response::new(RefreshTokenResponse {
            ..Default::default()
        }))
    }
}
