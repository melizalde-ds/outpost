use crate::proto::auth::v1::auth_service_server::AuthService;
use crate::proto::auth::v1::{
    LoginRequest, LoginResponse, LogoutRequest, LogoutResponse, RefreshTokenRequest,
    RefreshTokenResponse, RegisterRequest, RegisterResponse, ValidateTokenRequest,
    ValidateTokenResponse,
};
use tonic::{Request, Response, Status};

#[derive(Debug, Default)]
pub struct WeatherAuthService;

#[tonic::async_trait]
impl AuthService for WeatherAuthService {
    async fn register(
        &self,
        _request: Request<RegisterRequest>,
    ) -> Result<Response<RegisterResponse>, Status> {
        Ok(Response::new(RegisterResponse {
            user_id: "user123".to_string(),
        }))
    }

    async fn login(
        &self,
        _request: Request<LoginRequest>,
    ) -> Result<Response<LoginResponse>, Status> {
        Ok(Response::new(LoginResponse {
            access_token: "access_token_example".to_string(),
            refresh_token: "refresh_token_example".to_string(),
            ..Default::default()
        }))
    }

    async fn refresh_token(
        &self,
        _request: Request<RefreshTokenRequest>,
    ) -> Result<Response<RefreshTokenResponse>, Status> {
        Ok(Response::new(RefreshTokenResponse {
            access_token: "new_access_token_example".to_string(),
            refresh_token: "new_refresh_token_example".to_string(),
            ..Default::default()
        }))
    }

    async fn logout(
        &self,
        _request: Request<LogoutRequest>,
    ) -> Result<Response<LogoutResponse>, Status> {
        Ok(Response::new(LogoutResponse {}))
    }

    async fn validate_token(
        &self,
        _request: Request<ValidateTokenRequest>,
    ) -> Result<Response<ValidateTokenResponse>, Status> {
        Ok(Response::new(ValidateTokenResponse {
            ..Default::default()
        }))
    }
}
