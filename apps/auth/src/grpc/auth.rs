use tonic::{Request, Response, Status};

use crate::{
    proto::auth::v1::{
        LoginRequest, LoginResponse, RefreshTokenRequest, RefreshTokenResponse, RegisterRequest,
        RegisterResponse, auth_service_server::AuthService,
    },
    service::WeatherAuthService,
};

#[tonic::async_trait]
impl AuthService for WeatherAuthService {
    async fn register(
        &self,
        request: Request<RegisterRequest>,
    ) -> Result<Response<RegisterResponse>, Status> {
        Ok(Response::new(
            WeatherAuthService::register(self, request.into_inner()).await?,
        ))
    }

    async fn login(
        &self,
        request: Request<LoginRequest>,
    ) -> Result<Response<LoginResponse>, Status> {
        Ok(Response::new(
            WeatherAuthService::login(self, request.into_inner()).await?,
        ))
    }

    async fn refresh_token(
        &self,
        request: Request<RefreshTokenRequest>,
    ) -> Result<Response<RefreshTokenResponse>, Status> {
        Ok(Response::new(
            WeatherAuthService::refresh_token(self, request.into_inner()).await?,
        ))
    }
}
