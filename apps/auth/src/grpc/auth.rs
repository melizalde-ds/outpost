use chrono::{DateTime, Utc};
use prost_types::Timestamp;
use secrecy::SecretSlice;
use tonic::{Request, Response, Status};

use crate::{
    proto::auth::v1::{
        LoginRequest, LoginResponse, LogoutAllRequest, LogoutAllResponse, LogoutRequest,
        LogoutResponse, RefreshTokenRequest, RefreshTokenResponse, RegisterRequest,
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

    async fn logout(
        &self,
        request: Request<LogoutRequest>,
    ) -> Result<Response<LogoutResponse>, Status> {
        DefaultAuthService::logout(self, &request.into_inner().refresh_token).await?;

        Ok(Response::new(LogoutResponse {}))
    }

    async fn logout_all(
        &self,
        request: Request<LogoutAllRequest>,
    ) -> Result<Response<LogoutAllResponse>, Status> {
        DefaultAuthService::logout_all(self, &request.into_inner().refresh_token).await?;

        Ok(Response::new(LogoutAllResponse {}))
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::Duration;
    use pretty_assertions::assert_eq;
    use redis::{ErrorKind, RedisError};
    use rstest::rstest;
    use tonic::Code;
    use uuid::Uuid;

    use super::*;
    use crate::{
        jwk_cache::{JwkCacheError, LocalJwkSet, MockJwkCache},
        password::MockPasswordHasher,
        refresh_token::{Entry, MockRefreshTokenStore, RefreshToken, RefreshTokenStoreError},
        repository::{
            MockTokenFamilyRepository, MockUserRepository, RepositoryError, RevocationReason,
            TokenFamilyId, UserId,
        },
        settings::RefreshTokenSettings,
        signing::{MockSigningService, PublicKey, SignedToken, SigningError},
        test_support::{
            BuiltAuth, FakeSigner, InMemoryJwkCache, SIGNING_KEY_ID, TestAuth, start_time,
        },
    };

    const ALICE: &str = "alice";
    const BOB: &str = "bob";
    const PASSWORD: &str = "correct horse";
    const WRONG_PASSWORD: &str = "wrong horse";
    const GARBAGE_TOKEN: &str = "not-a-refresh-token";
    const USER: UserId = UserId(Uuid::from_u128(1));
    const FAMILY: TokenFamilyId = TokenFamilyId(Uuid::from_u128(1));

    async fn register(
        auth: &BuiltAuth,
        username: &str,
        password: &str,
    ) -> Result<TokenPair, Status> {
        let request = RegisterRequest {
            username: username.into(),
            password: password.into(),
        };
        AuthService::register(&auth.service, Request::new(request))
            .await
            .map(|response| response.into_inner().tokens.unwrap())
    }

    async fn login(auth: &BuiltAuth, username: &str, password: &str) -> Result<TokenPair, Status> {
        let request = LoginRequest {
            username: username.into(),
            password: password.into(),
        };
        AuthService::login(&auth.service, Request::new(request))
            .await
            .map(|response| response.into_inner().tokens.unwrap())
    }

    async fn refresh(auth: &BuiltAuth, refresh_token: &str) -> Result<TokenPair, Status> {
        let request = RefreshTokenRequest {
            refresh_token: refresh_token.into(),
        };
        AuthService::refresh_token(&auth.service, Request::new(request))
            .await
            .map(|response| response.into_inner().tokens.unwrap())
    }

    async fn logout(auth: &BuiltAuth, refresh_token: &str) -> Result<(), Status> {
        let request = LogoutRequest {
            refresh_token: refresh_token.into(),
        };
        AuthService::logout(&auth.service, Request::new(request))
            .await
            .map(|_| ())
    }

    async fn logout_all(auth: &BuiltAuth, refresh_token: &str) -> Result<(), Status> {
        let request = LogoutAllRequest {
            refresh_token: refresh_token.into(),
        };
        AuthService::logout_all(&auth.service, Request::new(request))
            .await
            .map(|_| ())
    }

    async fn sign_up(auth: &BuiltAuth, username: &str) -> String {
        register(auth, username, PASSWORD)
            .await
            .unwrap()
            .refresh_token
    }

    async fn sign_in(auth: &BuiltAuth, username: &str) -> String {
        login(auth, username, PASSWORD).await.unwrap().refresh_token
    }

    async fn refresh_after_days(auth: &BuiltAuth, refresh_token: &str, days: i64) -> String {
        auth.clock.advance(Duration::days(days));
        refresh(auth, refresh_token).await.unwrap().refresh_token
    }

    fn expect_code(result: Result<impl std::fmt::Debug, Status>, code: Code) {
        assert_eq!(result.unwrap_err().code(), code);
    }

    fn signer_returning(key_id: Uuid, new_key: Option<Uuid>) -> MockSigningService {
        let mut signer = MockSigningService::new();
        signer.expect_sign().returning(move |sub| {
            Ok(SignedToken {
                token: FakeSigner::token_for(UserId(sub)),
                key_id,
                new_key: new_key.map(|kid| PublicKey {
                    kid,
                    key: "new-public-key".into(),
                    exp: start_time() + Duration::days(1),
                }),
            })
        });
        signer.expect_get_verification_keys().never();
        signer
    }

    fn session_token() -> String {
        let (token, _) = RefreshToken::generate(
            USER,
            FAMILY,
            start_time() + Duration::days(7),
            start_time() + Duration::days(30),
        )
        .unwrap();
        token
    }

    fn live_session() -> Entry {
        Entry::Live {
            user_id: USER,
            family_exp: start_time() + Duration::days(30),
        }
    }

    fn store_finding(entry: Entry) -> MockRefreshTokenStore {
        let mut store = MockRefreshTokenStore::new();
        store
            .expect_find()
            .returning(move |_| Ok(Some(entry.clone())));
        store
    }

    fn families_with_the_session() -> MockTokenFamilyRepository {
        let mut families = MockTokenFamilyRepository::new();
        families.expect_is_live().returning(|_, _| Ok(true));
        families
    }

    fn active_user() -> MockUserRepository {
        let mut users = MockUserRepository::new();
        users.expect_is_active().returning(|_| Ok(true));
        users
    }

    fn redis_down() -> RedisError {
        RedisError::from((ErrorKind::Io, "down"))
    }

    fn database_down() -> RepositoryError {
        RepositoryError::from(sqlx::Error::PoolTimedOut)
    }

    #[test]
    fn to_timestamp_keeps_the_seconds_and_nanoseconds() {
        // Arrange
        let time = DateTime::from_timestamp(1_767_225_600, 250).unwrap();

        // Act
        let timestamp = to_timestamp(time);

        // Assert
        assert_eq!(
            timestamp,
            Timestamp {
                seconds: 1_767_225_600,
                nanos: 250,
            }
        );
    }

    mod register {
        use pretty_assertions::assert_eq;

        use super::*;

        #[tokio::test]
        async fn new_username_returns_an_access_token_for_the_new_user() {
            // Arrange
            let auth = TestAuth::new().build();

            // Act
            let tokens = register(&auth, ALICE, PASSWORD).await.unwrap();

            // Assert
            assert_eq!(
                tokens.access_token,
                FakeSigner::token_for(auth.users.user_id(ALICE))
            );
        }

        #[tokio::test]
        async fn new_username_returns_a_refresh_token_that_expires_after_7_days() {
            // Arrange
            let auth = TestAuth::new().build();

            // Act
            let tokens = register(&auth, ALICE, PASSWORD).await.unwrap();

            // Assert
            assert_eq!(
                tokens.refresh_token_expires_at,
                Some(to_timestamp(start_time() + Duration::days(7)))
            );
        }

        #[tokio::test]
        async fn taken_username_returns_already_exists() {
            // Arrange
            let auth = TestAuth::new().build();
            sign_up(&auth, ALICE).await;

            // Act
            let result = register(&auth, ALICE, "another password").await;

            // Assert
            expect_code(result, Code::AlreadyExists);
        }

        #[tokio::test]
        async fn empty_username_returns_invalid_argument() {
            // Arrange
            let auth = TestAuth::new().build();

            // Act
            let result = register(&auth, "", PASSWORD).await;

            // Assert
            expect_code(result, Code::InvalidArgument);
        }

        #[tokio::test]
        async fn username_of_only_spaces_returns_invalid_argument() {
            // Arrange
            let auth = TestAuth::new().build();

            // Act
            let result = register(&auth, "   ", PASSWORD).await;

            // Assert
            expect_code(result, Code::InvalidArgument);
        }

        #[tokio::test]
        async fn empty_password_returns_invalid_argument() {
            // Arrange
            let auth = TestAuth::new().build();

            // Act
            let result = register(&auth, ALICE, "").await;

            // Assert
            expect_code(result, Code::InvalidArgument);
        }

        #[rstest]
        #[case::username_at_the_limit(256, 8)]
        #[case::password_at_the_limit(5, 1024)]
        #[tokio::test]
        async fn at_the_length_limit_returns_tokens(
            #[case] username_chars: usize,
            #[case] password_bytes: usize,
        ) {
            // Arrange
            let auth = TestAuth::new().build();

            // Act
            let result = register(
                &auth,
                &"é".repeat(username_chars),
                &"p".repeat(password_bytes),
            )
            .await;

            // Assert
            assert!(result.is_ok());
        }

        #[rstest]
        #[case::username_too_long(257, 8)]
        #[case::password_too_long(5, 1025)]
        #[tokio::test]
        async fn over_the_length_limit_returns_invalid_argument(
            #[case] username_chars: usize,
            #[case] password_bytes: usize,
        ) {
            // Arrange
            let auth = TestAuth::new().build();

            // Act
            let result = register(
                &auth,
                &"é".repeat(username_chars),
                &"p".repeat(password_bytes),
            )
            .await;

            // Assert
            expect_code(result, Code::InvalidArgument);
        }

        #[tokio::test]
        async fn username_with_surrounding_spaces_is_the_same_user() {
            // Arrange
            let auth = TestAuth::new().build();
            sign_up(&auth, ALICE).await;

            // Act
            let result = register(&auth, " alice ", PASSWORD).await;

            // Assert
            expect_code(result, Code::AlreadyExists);
        }

        #[tokio::test]
        async fn idle_limit_longer_than_the_session_limit_returns_a_token_ending_with_the_session()
        {
            // Arrange
            let auth = TestAuth::new()
                .with_refresh_settings(RefreshTokenSettings {
                    idle_timeout_secs: 40 * 24 * 60 * 60,
                    max_age_secs: 30 * 24 * 60 * 60,
                })
                .build();

            // Act
            let tokens = register(&auth, ALICE, PASSWORD).await.unwrap();

            // Assert
            assert_eq!(
                tokens.refresh_token_expires_at,
                Some(to_timestamp(start_time() + Duration::days(30)))
            );
        }
    }

    mod login {
        use pretty_assertions::assert_eq;

        use super::*;

        #[tokio::test]
        async fn right_password_returns_an_access_token_for_the_user() {
            // Arrange
            let auth = TestAuth::new().build();
            sign_up(&auth, ALICE).await;

            // Act
            let tokens = login(&auth, ALICE, PASSWORD).await.unwrap();

            // Assert
            assert_eq!(
                tokens.access_token,
                FakeSigner::token_for(auth.users.user_id(ALICE))
            );
        }

        #[tokio::test]
        async fn wrong_password_returns_unauthenticated() {
            // Arrange
            let auth = TestAuth::new().build();
            sign_up(&auth, ALICE).await;

            // Act
            let result = login(&auth, ALICE, WRONG_PASSWORD).await;

            // Assert
            expect_code(result, Code::Unauthenticated);
        }

        #[tokio::test]
        async fn username_with_surrounding_spaces_logs_in() {
            // Arrange
            let auth = TestAuth::new().build();
            sign_up(&auth, ALICE).await;

            // Act
            let result = login(&auth, " alice ", PASSWORD).await;

            // Assert
            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn unknown_username_returns_the_same_error_as_a_wrong_password() {
            // Arrange
            let auth = TestAuth::new().build();
            sign_up(&auth, ALICE).await;
            let wrong_password = login(&auth, ALICE, WRONG_PASSWORD).await.unwrap_err();

            // Act
            let unknown_username = login(&auth, "nobody", PASSWORD).await.unwrap_err();

            // Assert
            assert_eq!(unknown_username.code(), wrong_password.code());
            assert_eq!(unknown_username.message(), wrong_password.message());
        }

        #[tokio::test]
        async fn unknown_username_still_checks_a_password() {
            // Arrange
            let mut passwords = MockPasswordHasher::new();
            passwords
                .expect_verify_dummy()
                .times(1)
                .returning(|_| Ok(()));
            let auth = TestAuth::new().with_passwords(passwords).build();

            // Act
            let _ = login(&auth, "nobody", PASSWORD).await;

            // Assert
            drop(auth);
        }

        #[rstest]
        #[case::username_too_long(257, 8)]
        #[case::password_too_long(5, 1025)]
        #[tokio::test]
        async fn too_long_returns_unauthenticated_without_checking_a_password(
            #[case] username_chars: usize,
            #[case] password_bytes: usize,
        ) {
            // Arrange
            let mut passwords = MockPasswordHasher::new();
            passwords.expect_verify().never();
            passwords.expect_verify_dummy().never();
            let auth = TestAuth::new().with_passwords(passwords).build();

            // Act
            let result = login(
                &auth,
                &"a".repeat(username_chars),
                &"p".repeat(password_bytes),
            )
            .await;

            // Assert
            expect_code(result, Code::Unauthenticated);
        }

        #[tokio::test]
        async fn switched_off_account_returns_unauthenticated() {
            // Arrange
            let auth = TestAuth::new().build();
            sign_up(&auth, ALICE).await;
            auth.users.set_active(ALICE, false);

            // Act
            let result = login(&auth, ALICE, PASSWORD).await;

            // Assert
            expect_code(result, Code::Unauthenticated);
        }
    }

    mod refresh {
        use pretty_assertions::{assert_eq, assert_ne};

        use super::*;

        #[tokio::test]
        async fn valid_token_returns_a_new_refresh_token() {
            // Arrange
            let auth = TestAuth::new().build();
            let token = sign_up(&auth, ALICE).await;

            // Act
            let tokens = refresh(&auth, &token).await.unwrap();

            // Assert
            assert_ne!(tokens.refresh_token, token);
        }

        #[tokio::test]
        async fn swapped_token_is_refused() {
            // Arrange
            let auth = TestAuth::new().build();
            let token = sign_up(&auth, ALICE).await;
            refresh(&auth, &token).await.unwrap();

            // Act
            let result = refresh(&auth, &token).await;

            // Assert
            expect_code(result, Code::Unauthenticated);
        }

        #[tokio::test]
        async fn swapped_token_used_again_ends_the_session() {
            // Arrange
            let auth = TestAuth::new().build();
            let first = sign_up(&auth, ALICE).await;
            let newest = refresh(&auth, &first).await.unwrap().refresh_token;
            let _ = refresh(&auth, &first).await;

            // Act
            let result = refresh(&auth, &newest).await;

            // Assert
            expect_code(result, Code::Unauthenticated);
        }

        #[tokio::test]
        async fn garbage_token_is_refused() {
            // Arrange
            let auth = TestAuth::new().build();

            // Act
            let result = refresh(&auth, GARBAGE_TOKEN).await;

            // Assert
            expect_code(result, Code::Unauthenticated);
        }

        #[tokio::test]
        async fn token_unused_for_7_days_is_refused() {
            // Arrange
            let auth = TestAuth::new().build();
            let token = sign_up(&auth, ALICE).await;
            auth.clock.advance(Duration::days(7));

            // Act
            let result = refresh(&auth, &token).await;

            // Assert
            expect_code(result, Code::Unauthenticated);
        }

        #[tokio::test]
        async fn token_used_just_before_7_days_works() {
            // Arrange
            let auth = TestAuth::new().build();
            let token = sign_up(&auth, ALICE).await;
            auth.clock.advance(Duration::days(7) - Duration::seconds(1));

            // Act
            let result = refresh(&auth, &token).await;

            // Assert
            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn busy_session_is_refused_after_30_days() {
            // Arrange
            let auth = TestAuth::new().build();
            let token = sign_up(&auth, ALICE).await;
            let token = refresh_after_days(&auth, &token, 6).await;
            let token = refresh_after_days(&auth, &token, 6).await;
            let token = refresh_after_days(&auth, &token, 6).await;
            let token = refresh_after_days(&auth, &token, 6).await;
            auth.clock.advance(Duration::days(6));

            // Act
            let result = refresh(&auth, &token).await;

            // Assert
            expect_code(result, Code::Unauthenticated);
        }

        #[tokio::test]
        async fn busy_session_works_just_before_30_days() {
            // Arrange
            let auth = TestAuth::new().build();
            let token = sign_up(&auth, ALICE).await;
            let token = refresh_after_days(&auth, &token, 6).await;
            let token = refresh_after_days(&auth, &token, 6).await;
            let token = refresh_after_days(&auth, &token, 6).await;
            let token = refresh_after_days(&auth, &token, 6).await;
            auth.clock.advance(Duration::days(6) - Duration::seconds(1));

            // Act
            let result = refresh(&auth, &token).await;

            // Assert
            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn new_token_never_expires_after_the_session_ends() {
            // Arrange
            let auth = TestAuth::new().build();
            let token = sign_up(&auth, ALICE).await;
            let token = refresh_after_days(&auth, &token, 6).await;
            let token = refresh_after_days(&auth, &token, 6).await;
            let token = refresh_after_days(&auth, &token, 6).await;
            auth.clock.advance(Duration::days(6));

            // Act
            let tokens = refresh(&auth, &token).await.unwrap();

            // Assert
            assert_eq!(
                tokens.refresh_token_expires_at,
                Some(to_timestamp(start_time() + Duration::days(30)))
            );
        }

        #[tokio::test]
        async fn switched_off_account_is_refused() {
            // Arrange
            let auth = TestAuth::new().build();
            let token = sign_up(&auth, ALICE).await;
            auth.users.set_active(ALICE, false);

            // Act
            let result = refresh(&auth, &token).await;

            // Assert
            expect_code(result, Code::Unauthenticated);
        }

        #[tokio::test]
        async fn switched_off_account_session_stays_ended_when_switched_back_on() {
            // Arrange
            let auth = TestAuth::new().build();
            let token = sign_up(&auth, ALICE).await;
            auth.users.set_active(ALICE, false);
            let _ = refresh(&auth, &token).await;
            auth.users.set_active(ALICE, true);

            // Act
            let result = refresh(&auth, &token).await;

            // Assert
            expect_code(result, Code::Unauthenticated);
        }

        #[tokio::test]
        async fn token_swapped_by_another_refresh_at_the_same_time_ends_the_session() {
            // Arrange
            let mut store = store_finding(live_session());
            store
                .expect_rotate()
                .returning(|_, _| Ok(Some(Entry::Rotated)));
            let mut families = families_with_the_session();
            families
                .expect_revoke()
                .withf(|family_id, reason| {
                    *family_id == FAMILY && matches!(reason, RevocationReason::Reuse)
                })
                .times(1)
                .returning(|_, _| Ok(()));
            let auth = TestAuth::new()
                .with_users(active_user())
                .with_families(families)
                .with_refresh_tokens(store)
                .build();

            // Act
            let result = refresh(&auth, &session_token()).await;

            // Assert
            expect_code(result, Code::Unauthenticated);
        }

        #[tokio::test]
        async fn token_expiring_during_the_refresh_is_refused_without_ending_the_session() {
            // Arrange
            let mut store = store_finding(live_session());
            store.expect_rotate().returning(|_, _| Ok(None));
            let mut families = families_with_the_session();
            families.expect_revoke().never();
            let auth = TestAuth::new()
                .with_users(active_user())
                .with_families(families)
                .with_refresh_tokens(store)
                .build();

            // Act
            let result = refresh(&auth, &session_token()).await;

            // Assert
            expect_code(result, Code::Unauthenticated);
        }
    }

    mod logout {
        use super::*;

        #[tokio::test]
        async fn logged_out_token_cannot_refresh() {
            // Arrange
            let auth = TestAuth::new().build();
            let token = sign_up(&auth, ALICE).await;
            logout(&auth, &token).await.unwrap();

            // Act
            let result = refresh(&auth, &token).await;

            // Assert
            expect_code(result, Code::Unauthenticated);
        }

        #[tokio::test]
        async fn garbage_token_returns_ok() {
            // Arrange
            let auth = TestAuth::new().build();

            // Act
            let result = logout(&auth, GARBAGE_TOKEN).await;

            // Assert
            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn same_token_twice_returns_ok() {
            // Arrange
            let auth = TestAuth::new().build();
            let token = sign_up(&auth, ALICE).await;
            logout(&auth, &token).await.unwrap();

            // Act
            let result = logout(&auth, &token).await;

            // Assert
            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn swapped_token_returns_ok() {
            // Arrange
            let auth = TestAuth::new().build();
            let token = sign_up(&auth, ALICE).await;
            refresh(&auth, &token).await.unwrap();

            // Act
            let result = logout(&auth, &token).await;

            // Assert
            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn swapped_token_ends_the_session() {
            // Arrange
            let auth = TestAuth::new().build();
            let first = sign_up(&auth, ALICE).await;
            let newest = refresh(&auth, &first).await.unwrap().refresh_token;
            logout(&auth, &first).await.unwrap();

            // Act
            let result = refresh(&auth, &newest).await;

            // Assert
            expect_code(result, Code::Unauthenticated);
        }

        #[tokio::test]
        async fn other_sessions_keep_working() {
            // Arrange
            let auth = TestAuth::new().build();
            let phone = sign_up(&auth, ALICE).await;
            let laptop = sign_in(&auth, ALICE).await;
            logout(&auth, &phone).await.unwrap();

            // Act
            let result = refresh(&auth, &laptop).await;

            // Assert
            assert!(result.is_ok());
        }
    }

    mod logout_all {
        use super::*;

        #[tokio::test]
        async fn ends_every_session_for_the_user() {
            // Arrange
            let auth = TestAuth::new().build();
            let phone = sign_up(&auth, ALICE).await;
            let laptop = sign_in(&auth, ALICE).await;
            logout_all(&auth, &phone).await.unwrap();

            // Act
            let result = refresh(&auth, &laptop).await;

            // Assert
            expect_code(result, Code::Unauthenticated);
        }

        #[tokio::test]
        async fn token_used_to_log_out_stops_working() {
            // Arrange
            let auth = TestAuth::new().build();
            let phone = sign_up(&auth, ALICE).await;
            logout_all(&auth, &phone).await.unwrap();

            // Act
            let result = refresh(&auth, &phone).await;

            // Assert
            expect_code(result, Code::Unauthenticated);
        }

        #[tokio::test]
        async fn other_users_sessions_keep_working() {
            // Arrange
            let auth = TestAuth::new().build();
            let alice = sign_up(&auth, ALICE).await;
            let bob = sign_up(&auth, BOB).await;
            logout_all(&auth, &alice).await.unwrap();

            // Act
            let result = refresh(&auth, &bob).await;

            // Assert
            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn garbage_token_returns_unauthenticated() {
            // Arrange
            let auth = TestAuth::new().build();

            // Act
            let result = logout_all(&auth, GARBAGE_TOKEN).await;

            // Assert
            expect_code(result, Code::Unauthenticated);
        }

        #[tokio::test]
        async fn garbage_token_ends_no_sessions() {
            // Arrange
            let auth = TestAuth::new().build();
            let token = sign_up(&auth, ALICE).await;
            let _ = logout_all(&auth, GARBAGE_TOKEN).await;

            // Act
            let result = refresh(&auth, &token).await;

            // Assert
            assert!(result.is_ok());
        }
    }

    mod signing_keys {
        use pretty_assertions::assert_eq;

        use super::*;

        const NEW_KEY_ID: Uuid = Uuid::from_u128(0x7e3);

        #[tokio::test]
        async fn unknown_key_is_fetched_and_saved_for_apps() {
            // Arrange
            let keys = Arc::new(InMemoryJwkCache::default());
            let auth = TestAuth::new().with_jwk_cache(keys.clone()).build();

            // Act
            register(&auth, ALICE, PASSWORD).await.unwrap();

            // Assert
            assert!(keys.has_key(&SIGNING_KEY_ID));
        }

        #[tokio::test]
        async fn brand_new_key_is_saved_for_apps_without_fetching() {
            // Arrange
            let keys = Arc::new(InMemoryJwkCache::default());
            let auth = TestAuth::new()
                .with_signer(signer_returning(NEW_KEY_ID, Some(NEW_KEY_ID)))
                .with_jwk_cache(keys.clone())
                .build();

            // Act
            register(&auth, ALICE, PASSWORD).await.unwrap();

            // Assert
            assert!(keys.has_key(&NEW_KEY_ID));
        }

        fn local_kids(auth: &BuiltAuth) -> Option<Vec<Uuid>> {
            auth.service
                .live_jwks()
                .map(|keys| keys.into_iter().map(|k| k.kid).collect())
        }

        #[tokio::test]
        async fn fetched_keys_are_kept_locally() {
            // Arrange
            let auth = TestAuth::new().build();

            // Act
            register(&auth, ALICE, PASSWORD).await.unwrap();

            // Assert
            assert_eq!(local_kids(&auth), Some(vec![SIGNING_KEY_ID]));
        }

        #[tokio::test]
        async fn brand_new_key_is_added_to_the_local_keys() {
            // Arrange
            let local_jwks = Arc::new(LocalJwkSet::default());
            local_jwks.replace(vec![]);
            let auth = TestAuth::new()
                .with_signer(signer_returning(NEW_KEY_ID, Some(NEW_KEY_ID)))
                .with_local_jwks(local_jwks)
                .build();

            // Act
            register(&auth, ALICE, PASSWORD).await.unwrap();

            // Assert
            assert_eq!(local_kids(&auth), Some(vec![NEW_KEY_ID]));
        }

        #[tokio::test]
        async fn known_key_is_not_fetched_again() {
            // Arrange
            let keys = Arc::new(InMemoryJwkCache::with_key(SIGNING_KEY_ID));
            let auth = TestAuth::new()
                .with_signer(signer_returning(SIGNING_KEY_ID, None))
                .with_jwk_cache(keys)
                .build();

            // Act
            let result = register(&auth, ALICE, PASSWORD).await;

            // Assert
            assert!(result.is_ok());
        }
    }

    mod outages {
        use pretty_assertions::assert_eq;

        use super::*;

        #[tokio::test]
        async fn key_provider_unreachable_returns_unavailable() {
            // Arrange
            let mut signer = MockSigningService::new();
            signer
                .expect_sign()
                .returning(|_| Err(SigningError::Call(Status::unavailable("down"))));
            let auth = TestAuth::new().with_signer(signer).build();

            // Act
            let result = register(&auth, ALICE, PASSWORD).await;

            // Assert
            expect_code(result, Code::Unavailable);
        }

        #[tokio::test]
        async fn key_provider_timing_out_returns_unavailable() {
            // Arrange
            let mut signer = MockSigningService::new();
            signer
                .expect_sign()
                .returning(|_| Err(SigningError::Call(Status::cancelled("Timeout expired"))));
            let auth = TestAuth::new().with_signer(signer).build();

            // Act
            let result = register(&auth, ALICE, PASSWORD).await;

            // Assert
            expect_code(result, Code::Unavailable);
        }

        #[tokio::test]
        async fn key_provider_failing_returns_internal() {
            // Arrange
            let mut signer = MockSigningService::new();
            signer
                .expect_sign()
                .returning(|_| Err(SigningError::Call(Status::internal("boom"))));
            let auth = TestAuth::new().with_signer(signer).build();

            // Act
            let result = register(&auth, ALICE, PASSWORD).await;

            // Assert
            expect_code(result, Code::Internal);
        }

        #[tokio::test]
        async fn key_provider_unreachable_on_register_tells_the_user_to_log_in() {
            // Arrange
            let mut signer = MockSigningService::new();
            signer
                .expect_sign()
                .returning(|_| Err(SigningError::Call(Status::unavailable("down"))));
            let auth = TestAuth::new().with_signer(signer).build();

            // Act
            let status = register(&auth, ALICE, PASSWORD).await.unwrap_err();

            // Assert
            assert_eq!(
                status.message(),
                "account created, but sign-in failed; please log in"
            );
        }

        #[tokio::test]
        async fn public_key_store_unreachable_returns_unavailable() {
            // Arrange
            let mut keys = MockJwkCache::new();
            keys.expect_contains().returning(|_| {
                Err(JwkCacheError::Command(RedisError::from((
                    ErrorKind::Io,
                    "down",
                ))))
            });
            let auth = TestAuth::new().with_jwk_cache(Arc::new(keys)).build();

            // Act
            let result = register(&auth, ALICE, PASSWORD).await;

            // Assert
            expect_code(result, Code::Unavailable);
        }

        #[tokio::test]
        async fn database_unreachable_on_register_returns_unavailable() {
            // Arrange
            let mut users = MockUserRepository::new();
            users
                .expect_insert_credentials()
                .returning(|_, _| Err(database_down()));
            let auth = TestAuth::new().with_users(users).build();

            // Act
            let result = register(&auth, ALICE, PASSWORD).await;

            // Assert
            expect_code(result, Code::Unavailable);
        }

        #[tokio::test]
        async fn refresh_token_store_unreachable_returns_unavailable() {
            // Arrange
            let mut store = MockRefreshTokenStore::new();
            store
                .expect_find()
                .returning(|_| Err(RefreshTokenStoreError::Command(redis_down())));
            let auth = TestAuth::new().with_refresh_tokens(store).build();

            // Act
            let result = refresh(&auth, &session_token()).await;

            // Assert
            expect_code(result, Code::Unavailable);
        }

        #[tokio::test]
        async fn database_unreachable_when_checking_the_user_on_refresh_returns_unavailable() {
            // Arrange
            let mut users = MockUserRepository::new();
            users.expect_is_active().returning(|_| Err(database_down()));
            let auth = TestAuth::new()
                .with_users(users)
                .with_families(families_with_the_session())
                .with_refresh_tokens(store_finding(live_session()))
                .build();

            // Act
            let result = refresh(&auth, &session_token()).await;

            // Assert
            expect_code(result, Code::Unavailable);
        }

        #[tokio::test]
        async fn database_unreachable_when_ending_a_reused_session_returns_unavailable() {
            // Arrange
            let mut families = MockTokenFamilyRepository::new();
            families
                .expect_revoke()
                .returning(|_, _| Err(database_down()));
            let auth = TestAuth::new()
                .with_families(families)
                .with_refresh_tokens(store_finding(Entry::Rotated))
                .build();

            // Act
            let result = refresh(&auth, &session_token()).await;

            // Assert
            expect_code(result, Code::Unavailable);
        }
    }
}
