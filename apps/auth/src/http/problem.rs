use axum::{
    extract::rejection::JsonRejection,
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use serde_json::json;
use tonic::{Code, Status};

use crate::service::AuthServiceError;

/// An RFC 9457 `application/problem+json` error.
#[derive(Debug)]
pub struct Problem {
    status: StatusCode,
    detail: String,
}

impl Problem {
    pub fn new(status: StatusCode, detail: impl Into<String>) -> Self {
        Self {
            status,
            detail: detail.into(),
        }
    }
}

// Going through the gRPC Status keeps the logging and client messages identical on both APIs.
impl From<AuthServiceError> for Problem {
    fn from(err: AuthServiceError) -> Self {
        Status::from(err).into()
    }
}

impl From<Status> for Problem {
    fn from(status: Status) -> Self {
        let http_status = match status.code() {
            Code::InvalidArgument => StatusCode::BAD_REQUEST,
            Code::Unauthenticated => StatusCode::UNAUTHORIZED,
            Code::AlreadyExists => StatusCode::CONFLICT,
            Code::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        Self::new(http_status, status.message())
    }
}

impl From<JsonRejection> for Problem {
    fn from(rejection: JsonRejection) -> Self {
        // A fixed detail: serde's message can echo parts of the request body.
        Self::new(
            rejection.status(),
            "request body must be a JSON object with the expected fields",
        )
    }
}

impl IntoResponse for Problem {
    fn into_response(self) -> Response {
        let body = json!({
            "type": "about:blank",
            "title": self.status.canonical_reason().unwrap_or("Error"),
            "status": self.status.as_u16(),
            "detail": self.detail,
        });
        let mut response = (
            self.status,
            [(header::CONTENT_TYPE, "application/problem+json")],
            body.to_string(),
        )
            .into_response();
        // HTTP expects a challenge on every 401; Bearer is the scheme of our access tokens.
        if self.status == StatusCode::UNAUTHORIZED {
            response
                .headers_mut()
                .insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
        }
        response
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::invalid_argument(Code::InvalidArgument, StatusCode::BAD_REQUEST)]
    #[case::unauthenticated(Code::Unauthenticated, StatusCode::UNAUTHORIZED)]
    #[case::already_exists(Code::AlreadyExists, StatusCode::CONFLICT)]
    #[case::unavailable(Code::Unavailable, StatusCode::SERVICE_UNAVAILABLE)]
    #[case::internal(Code::Internal, StatusCode::INTERNAL_SERVER_ERROR)]
    #[case::unknown(Code::Unknown, StatusCode::INTERNAL_SERVER_ERROR)]
    fn from_status_code_maps_to_http_status(#[case] code: Code, #[case] expected: StatusCode) {
        // Act
        let problem = Problem::from(Status::new(code, "detail"));

        // Assert
        assert_eq!(problem.status, expected);
    }

    #[rstest]
    #[case::unauthorized(StatusCode::UNAUTHORIZED, Some("Bearer"))]
    #[case::bad_request(StatusCode::BAD_REQUEST, None)]
    #[case::unavailable(StatusCode::SERVICE_UNAVAILABLE, None)]
    fn into_response_sets_www_authenticate_only_on_401(
        #[case] status: StatusCode,
        #[case] expected: Option<&'static str>,
    ) {
        // Arrange
        let problem = Problem::new(status, "detail");

        // Act
        let response = problem.into_response();

        // Assert
        assert_eq!(
            response.headers().get(header::WWW_AUTHENTICATE),
            expected.map(HeaderValue::from_static).as_ref()
        );
    }
}
