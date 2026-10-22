use std::time::Duration;

use tokio::time::{MissedTickBehavior, interval};
use tokio_util::sync::CancellationToken;
use tracing::warn;

use crate::{
    jwk_cache::JwkRecord,
    service::{AuthServiceError, DefaultAuthService},
};

impl DefaultAuthService {
    pub(super) async fn fetch_jwks(&self) -> Result<Vec<JwkRecord>, AuthServiceError> {
        Ok(self
            .signer
            .get_verification_keys()
            .await?
            .into_iter()
            .map(JwkRecord::try_from)
            .collect::<Result<_, _>>()?)
    }

    /// Reloads the local key copy now and then every `every`, until `shutdown`.
    /// A failed reload keeps the last good copy.
    pub(crate) async fn keep_jwks_fresh(&self, every: Duration, shutdown: CancellationToken) {
        let mut ticks = interval(every);
        ticks.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                biased;
                _ = shutdown.cancelled() => return,
                _ = ticks.tick() => match self.fetch_jwks().await {
                    Ok(records) => self.local_jwks.replace(records),
                    Err(e) => warn!(error = %e, "reloading public keys failed, keeping the last good copy"),
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use mockall::Sequence;
    use pretty_assertions::assert_eq;
    use tonic::Status;
    use uuid::Uuid;

    use super::*;
    use crate::{
        signing::{MockSigningService, PublicKey, SigningError},
        test_support::{BuiltAuth, SIGNING_KEY_ID, TestAuth, start_time},
    };

    const EVERY: Duration = Duration::from_secs(60);

    fn signing_key() -> PublicKey {
        PublicKey {
            kid: SIGNING_KEY_ID,
            key: "test-public-key".into(),
            exp: start_time() + chrono::Duration::days(1),
        }
    }

    fn key_provider_down() -> SigningError {
        SigningError::Call(Status::unavailable("down"))
    }

    async fn run_for(auth: &BuiltAuth, duration: Duration) {
        let shutdown = CancellationToken::new();
        let stop = async {
            tokio::time::sleep(duration).await;
            shutdown.cancel();
        };
        let run =
            async { tokio::join!(auth.service.keep_jwks_fresh(EVERY, shutdown.clone()), stop) };
        tokio::time::timeout(duration + EVERY, run)
            .await
            .expect("keep_jwks_fresh did not stop on shutdown");
    }

    fn local_kids(auth: &BuiltAuth) -> Option<Vec<Uuid>> {
        auth.service
            .live_jwks()
            .map(|keys| keys.into_iter().map(|k| k.kid).collect())
    }

    #[tokio::test(start_paused = true)]
    async fn keep_jwks_fresh_reloads_right_away_and_then_every_interval() {
        // Arrange
        let mut signer = MockSigningService::new();
        signer
            .expect_get_verification_keys()
            .times(3)
            .returning(|| Ok(vec![signing_key()]));
        let auth = TestAuth::new().with_signer(signer).build();

        // Act
        run_for(&auth, EVERY * 5 / 2).await;

        // Assert
        assert_eq!(local_kids(&auth), Some(vec![SIGNING_KEY_ID]));
    }

    #[tokio::test(start_paused = true)]
    async fn keep_jwks_fresh_key_provider_down_keeps_the_last_good_keys() {
        // Arrange
        let mut order = Sequence::new();
        let mut signer = MockSigningService::new();
        signer
            .expect_get_verification_keys()
            .times(1)
            .in_sequence(&mut order)
            .returning(|| Ok(vec![signing_key()]));
        signer
            .expect_get_verification_keys()
            .times(1)
            .in_sequence(&mut order)
            .returning(|| Err(key_provider_down()));
        let auth = TestAuth::new().with_signer(signer).build();

        // Act
        run_for(&auth, EVERY * 3 / 2).await;

        // Assert
        assert_eq!(local_kids(&auth), Some(vec![SIGNING_KEY_ID]));
    }

    #[tokio::test(start_paused = true)]
    async fn keep_jwks_fresh_key_provider_down_from_the_start_leaves_keys_not_loaded() {
        // Arrange
        let mut signer = MockSigningService::new();
        signer
            .expect_get_verification_keys()
            .returning(|| Err(key_provider_down()));
        let auth = TestAuth::new().with_signer(signer).build();

        // Act
        run_for(&auth, EVERY / 2).await;

        // Assert
        assert_eq!(local_kids(&auth), None);
    }

    #[tokio::test(start_paused = true)]
    async fn keep_jwks_fresh_after_shutdown_stops_without_reloading() {
        // Arrange
        let mut signer = MockSigningService::new();
        signer.expect_get_verification_keys().never();
        let auth = TestAuth::new().with_signer(signer).build();
        let shutdown = CancellationToken::new();
        shutdown.cancel();

        // Act
        auth.service.keep_jwks_fresh(EVERY, shutdown).await;

        // Assert
        assert_eq!(local_kids(&auth), None);
    }
}
