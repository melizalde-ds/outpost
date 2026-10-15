use std::fmt;

pub mod auth {
    pub mod v1 {
        tonic::include_proto!("outpost.auth.v1");
    }
}

pub mod key {
    pub mod v1 {
        tonic::include_proto!("outpost.key.v1");
    }
}

const REDACTED: &str = "[REDACTED]";

impl fmt::Debug for auth::v1::RegisterRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RegisterRequest")
            .field("username", &self.username)
            .field("password", &REDACTED)
            .finish()
    }
}

impl fmt::Debug for auth::v1::LoginRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LoginRequest")
            .field("username", &self.username)
            .field("password", &REDACTED)
            .finish()
    }
}

impl fmt::Debug for auth::v1::TokenPair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TokenPair")
            .field("access_token", &REDACTED)
            .field("refresh_token", &REDACTED)
            .field("refresh_token_expires_at", &self.refresh_token_expires_at)
            .finish()
    }
}

impl fmt::Debug for auth::v1::RefreshTokenRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RefreshTokenRequest")
            .field("refresh_token", &REDACTED)
            .finish()
    }
}

impl fmt::Debug for auth::v1::LogoutRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LogoutRequest")
            .field("refresh_token", &REDACTED)
            .finish()
    }
}

impl fmt::Debug for auth::v1::LogoutAllRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LogoutAllRequest")
            .field("refresh_token", &REDACTED)
            .finish()
    }
}

impl fmt::Debug for key::v1::SignTokenResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SignTokenResponse")
            .field("token", &REDACTED)
            .field("key_id", &self.key_id)
            .field("verification_key", &self.verification_key)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        auth::v1::{
            LoginRequest, LogoutAllRequest, LogoutRequest, RefreshTokenRequest, RegisterRequest,
            TokenPair,
        },
        key::v1::SignTokenResponse,
    };

    const PASSWORD: &str = "hunter2";
    const ACCESS_TOKEN: &str = "access-token-value";
    const REFRESH_TOKEN: &str = "refresh-token-value";

    #[test]
    fn login_request_debug_hides_the_password() {
        // Arrange
        let request = LoginRequest {
            username: "alice".into(),
            password: PASSWORD.into(),
        };

        // Act
        let output = format!("{request:?}");

        // Assert
        assert!(!output.contains(PASSWORD));
    }

    #[test]
    fn register_request_debug_hides_the_password() {
        // Arrange
        let request = RegisterRequest {
            username: "alice".into(),
            password: PASSWORD.into(),
        };

        // Act
        let output = format!("{request:?}");

        // Assert
        assert!(!output.contains(PASSWORD));
    }

    #[test]
    fn token_pair_debug_hides_both_tokens() {
        // Arrange
        let tokens = TokenPair {
            access_token: ACCESS_TOKEN.into(),
            refresh_token: REFRESH_TOKEN.into(),
            refresh_token_expires_at: None,
        };

        // Act
        let output = format!("{tokens:?}");

        // Assert
        assert!(!output.contains(ACCESS_TOKEN));
        assert!(!output.contains(REFRESH_TOKEN));
    }

    #[test]
    fn refresh_token_request_debug_hides_the_token() {
        // Arrange
        let request = RefreshTokenRequest {
            refresh_token: REFRESH_TOKEN.into(),
        };

        // Act
        let output = format!("{request:?}");

        // Assert
        assert!(!output.contains(REFRESH_TOKEN));
    }

    #[test]
    fn logout_request_debug_hides_the_token() {
        // Arrange
        let request = LogoutRequest {
            refresh_token: REFRESH_TOKEN.into(),
        };

        // Act
        let output = format!("{request:?}");

        // Assert
        assert!(!output.contains(REFRESH_TOKEN));
    }

    #[test]
    fn logout_all_request_debug_hides_the_token() {
        // Arrange
        let request = LogoutAllRequest {
            refresh_token: REFRESH_TOKEN.into(),
        };

        // Act
        let output = format!("{request:?}");

        // Assert
        assert!(!output.contains(REFRESH_TOKEN));
    }

    #[test]
    fn sign_token_response_debug_hides_the_token() {
        // Arrange
        let response = SignTokenResponse {
            token: ACCESS_TOKEN.into(),
            key_id: "key-id".into(),
            verification_key: None,
        };

        // Act
        let output = format!("{response:?}");

        // Assert
        assert!(!output.contains(ACCESS_TOKEN));
    }
}
