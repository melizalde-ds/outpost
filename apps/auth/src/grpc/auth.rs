use chrono::{DateTime, Utc};
use prost_types::Timestamp;
use secrecy::SecretSlice;
use tonic::{Request, Response, Status};

use crate::{
    proto::auth::v1::{
        LoginRequest, LoginResponse, RefreshTokenRequest, RefreshTokenResponse, RegisterRequest,
        RegisterResponse, TokenPair, auth_service_server::AuthService,
    },
    service::{DefaultAuthService, Tokens},
};

#[tonic::async_trait]
impl AuthService for DefaultAuthService {
    async fn register(
        &self,
        request: Request<RegisterRequest>,
    ) -> Result<Response<RegisterResponse>, Status> {
        let req = request.into_inner();
        let password = SecretSlice::from(req.password.into_bytes());
        let tokens = DefaultAuthService::register(self, req.username, password).await?;

        Ok(Response::new(RegisterResponse {
            tokens: Some(tokens.into()),
        }))
    }

    async fn login(
        &self,
        request: Request<LoginRequest>,
    ) -> Result<Response<LoginResponse>, Status> {
        let req = request.into_inner();
        let password = SecretSlice::from(req.password.into_bytes());
        let tokens = DefaultAuthService::login(self, req.username, password).await?;

        Ok(Response::new(LoginResponse {
            tokens: Some(tokens.into()),
        }))
    }

    async fn refresh_token(
        &self,
        request: Request<RefreshTokenRequest>,
    ) -> Result<Response<RefreshTokenResponse>, Status> {
        let tokens = self.refresh(&request.into_inner().refresh_token).await?;

        Ok(Response::new(RefreshTokenResponse {
            tokens: Some(tokens.into()),
        }))
    }
}

impl From<Tokens> for TokenPair {
    fn from(tokens: Tokens) -> Self {
        Self {
            access_token: tokens.access_token,
            refresh_token: tokens.refresh_token,
            refresh_token_expires_at: Some(to_timestamp(tokens.refresh_token_expires_at)),
        }
    }
}

fn to_timestamp(dt: DateTime<Utc>) -> Timestamp {
    Timestamp {
        seconds: dt.timestamp(),
        nanos: dt.timestamp_subsec_nanos() as i32,
    }
}
