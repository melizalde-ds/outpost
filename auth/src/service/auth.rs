use std::sync::Arc;

use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::{
    proto::auth::v1::{
        LoginRequest, LoginResponse, RefreshTokenRequest, RefreshTokenResponse, RegisterRequest,
        RegisterResponse, auth_service_server::AuthService,
    },
    repository::{auth::AuthRepository, errors::RepositoryError},
    service::errors::AuthServiceError,
};

#[derive(Debug)]
pub struct WeatherAuthService {
    pub repo: Arc<dyn AuthRepository>,
}

impl WeatherAuthService {
    async fn register_inner(
        &self,
        req: RegisterRequest,
    ) -> Result<RegisterResponse, AuthServiceError> {
        let repo = self.repo.clone();

        match repo.insert_credentials(&req.username, req.password).await {
            Ok(_) => Ok(RegisterResponse::default()),
            Err(RepositoryError::UniqueViolation(_)) => {
                Err(AuthServiceError::UserAlreadyExists(req.username))
            }
            Err(e) => Err(e.into()),
        }
    }

    async fn login_inner(&self, req: LoginRequest) -> Result<LoginResponse, AuthServiceError> {
        let repo = self.repo.clone();

        let credentials = repo
            .get_credentials(&req.username)
            .await?
            .ok_or(AuthServiceError::UserNotFound(req.username))?;

        if !credentials.active {
            return Err(AuthServiceError::UserNotActive(credentials.user_id));
        }

        Ok(LoginResponse::default())
    }

    async fn refresh_token_inner(
        &self,
        _req: RefreshTokenRequest,
    ) -> Result<RefreshTokenResponse, AuthServiceError> {
        let repo = self.repo.clone();
        let user_id = Uuid::new_v4();

        let active = repo
            .verify(&user_id)
            .await?
            .ok_or(AuthServiceError::UserNotFound(user_id.to_string()))?;

        if !active.0 {
            return Err(AuthServiceError::UserNotActive(user_id));
        }

        Ok(RefreshTokenResponse::default())
    }
}

#[tonic::async_trait]
impl AuthService for WeatherAuthService {
    async fn register(
        &self,
        request: Request<RegisterRequest>,
    ) -> Result<Response<RegisterResponse>, Status> {
        Ok(Response::new(
            self.register_inner(request.into_inner()).await?,
        ))
    }

    async fn login(
        &self,
        request: Request<LoginRequest>,
    ) -> Result<Response<LoginResponse>, Status> {
        Ok(Response::new(self.login_inner(request.into_inner()).await?))
    }

    async fn refresh_token(
        &self,
        request: Request<RefreshTokenRequest>,
    ) -> Result<Response<RefreshTokenResponse>, Status> {
        Ok(Response::new(
            self.refresh_token_inner(request.into_inner()).await?,
        ))
    }
}
