use crate::proto::auth::v1::auth_service_server::AuthService;
use crate::proto::auth::v1::{
    LoginRequest, LoginResponse, RefreshTokenRequest, RefreshTokenResponse, ValidateTokenRequest,
    ValidateTokenResponse,
};
use tonic::{Request, Response, Status};

#[derive(Debug, Default)]
pub struct WeatherAuthService;

#[tonic::async_trait]
impl AuthService for WeatherAuthService {
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

    async fn validate_token(
        &self,
        _request: Request<ValidateTokenRequest>,
    ) -> Result<Response<ValidateTokenResponse>, Status> {
        Ok(Response::new(ValidateTokenResponse {
            ..Default::default()
        }))
    }
}
