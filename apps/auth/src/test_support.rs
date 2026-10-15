use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use chrono::{DateTime, Duration, TimeZone, Utc};
use secrecy::{ExposeSecret, SecretSlice};
use uuid::Uuid;

use crate::{
    clock::Clock,
    jwk_cache::{JwkCache, JwkCacheError, JwkRecord},
    password::{PasswordError, PasswordHasher},
    refresh_token::{Entry, RefreshToken, RefreshTokenStore, RefreshTokenStoreError},
    repository::{
        Credentials, RepositoryError, RevocationReason, TokenFamilyId, TokenFamilyRepository,
        UserId, UserRepository,
    },
    service::DefaultAuthService,
    settings::RefreshTokenSettings,
    signing::{PublicKey, SignedToken, SigningError, SigningService},
};

pub fn start_time() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
}

pub const SIGNING_KEY_ID: Uuid = Uuid::from_u128(0x5160);

#[derive(Debug)]
pub struct ManualClock(Mutex<DateTime<Utc>>);

impl ManualClock {
    pub fn new(now: DateTime<Utc>) -> Self {
        Self(Mutex::new(now))
    }

    pub fn advance(&self, by: Duration) {
        *self.0.lock().unwrap() += by;
    }
}

impl Clock for ManualClock {
    fn now(&self) -> DateTime<Utc> {
        *self.0.lock().unwrap()
    }
}

#[derive(Debug)]
struct StoredUser {
    user_id: UserId,
    username: String,
    password_hash: String,
    active: bool,
}

#[derive(Debug, Default)]
pub struct InMemoryUsers(Mutex<Vec<StoredUser>>);

impl InMemoryUsers {
    pub fn set_active(&self, username: &str, active: bool) {
        let mut users = self.0.lock().unwrap();
        let user = users.iter_mut().find(|u| u.username == username).unwrap();
        user.active = active;
    }

    pub fn user_id(&self, username: &str) -> UserId {
        let users = self.0.lock().unwrap();
        users
            .iter()
            .find(|u| u.username == username)
            .unwrap()
            .user_id
    }
}

#[async_trait]
impl UserRepository for InMemoryUsers {
    async fn insert_credentials(
        &self,
        username: &str,
        password_hash: String,
    ) -> Result<UserId, RepositoryError> {
        let mut users = self.0.lock().unwrap();
        if users.iter().any(|u| u.username == username) {
            return Err(RepositoryError::UniqueViolation("auth_username_key".into()));
        }
        let user_id = UserId(Uuid::from_u128(users.len() as u128 + 1));
        users.push(StoredUser {
            user_id,
            username: username.to_owned(),
            password_hash,
            active: true,
        });
        Ok(user_id)
    }

    async fn get_credentials(
        &self,
        username: &str,
    ) -> Result<Option<Credentials>, RepositoryError> {
        let users = self.0.lock().unwrap();
        Ok(users
            .iter()
            .find(|u| u.username == username)
            .map(|u| Credentials {
                user_id: u.user_id,
                password_hash: u.password_hash.clone(),
                active: u.active,
            }))
    }

    async fn is_active(&self, user_id: &UserId) -> Result<bool, RepositoryError> {
        let users = self.0.lock().unwrap();
        Ok(users.iter().any(|u| u.user_id == *user_id && u.active))
    }
}

#[derive(Debug)]
struct StoredFamily {
    family_id: TokenFamilyId,
    user_id: UserId,
    expires_at: DateTime<Utc>,
    revoked: bool,
}

#[derive(Debug)]
pub struct InMemoryFamilies {
    clock: Arc<ManualClock>,
    families: Mutex<Vec<StoredFamily>>,
}

impl InMemoryFamilies {
    pub fn new(clock: Arc<ManualClock>) -> Self {
        Self {
            clock,
            families: Mutex::default(),
        }
    }

    fn is_live(&self, family: &StoredFamily) -> bool {
        !family.revoked && family.expires_at > self.clock.now()
    }
}

#[async_trait]
impl TokenFamilyRepository for InMemoryFamilies {
    async fn insert(
        &self,
        user_id: &UserId,
        expires_at: DateTime<Utc>,
    ) -> Result<TokenFamilyId, RepositoryError> {
        let mut families = self.families.lock().unwrap();
        let family_id = TokenFamilyId(Uuid::from_u128(families.len() as u128 + 1));
        families.push(StoredFamily {
            family_id,
            user_id: *user_id,
            expires_at,
            revoked: false,
        });
        Ok(family_id)
    }

    async fn find_unrevoked_by_user_id(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<TokenFamilyId>, RepositoryError> {
        let families = self.families.lock().unwrap();
        Ok(families
            .iter()
            .filter(|f| f.user_id == *user_id && self.is_live(f))
            .map(|f| f.family_id)
            .collect())
    }

    async fn revoke(
        &self,
        family_id: &TokenFamilyId,
        _reason: RevocationReason,
    ) -> Result<bool, RepositoryError> {
        let mut families = self.families.lock().unwrap();
        let Some(family) = families.iter_mut().find(|f| f.family_id == *family_id) else {
            return Ok(false);
        };
        let newly_revoked = !family.revoked;
        family.revoked = true;
        Ok(newly_revoked)
    }

    async fn revoke_all(
        &self,
        user_id: &UserId,
        _reason: RevocationReason,
    ) -> Result<Vec<TokenFamilyId>, RepositoryError> {
        let now = self.clock.now();
        let mut families = self.families.lock().unwrap();
        let mut revoked = Vec::new();
        for family in families.iter_mut() {
            if family.user_id == *user_id && !family.revoked && family.expires_at > now {
                family.revoked = true;
                revoked.push(family.family_id);
            }
        }
        Ok(revoked)
    }
}

type StoredEntry = (Entry, DateTime<Utc>);

// Like the Redis store, rotate marks the old token used and saves the new one in one step.
#[derive(Debug)]
pub struct InMemoryRefreshTokens {
    clock: Arc<ManualClock>,
    entries: Mutex<HashMap<[u8; 32], StoredEntry>>,
}

impl InMemoryRefreshTokens {
    pub fn new(clock: Arc<ManualClock>) -> Self {
        Self {
            clock,
            entries: Mutex::default(),
        }
    }

    fn live_entry(&self, hash: &[u8; 32]) -> Option<Entry> {
        let entries = self.entries.lock().unwrap();
        entries
            .get(hash)
            .filter(|(_, expires_at)| *expires_at > self.clock.now())
            .map(|(entry, _)| entry.clone())
    }

    fn save(&self, token: &RefreshToken) {
        let entry = Entry::Live {
            user_id: token.user_id,
            family_exp: token.family_exp,
        };
        self.entries
            .lock()
            .unwrap()
            .insert(token.hash, (entry, token.exp));
    }
}

#[async_trait]
impl RefreshTokenStore for InMemoryRefreshTokens {
    async fn insert(&self, token: &RefreshToken) -> Result<(), RefreshTokenStoreError> {
        self.save(token);
        Ok(())
    }

    async fn find(&self, hash: &[u8; 32]) -> Result<Option<Entry>, RefreshTokenStoreError> {
        Ok(self.live_entry(hash))
    }

    async fn rotate(
        &self,
        old: &[u8; 32],
        new: &RefreshToken,
    ) -> Result<Option<Entry>, RefreshTokenStoreError> {
        let previous = self.live_entry(old);
        if previous.is_some() {
            let mut entries = self.entries.lock().unwrap();
            let (entry, _) = entries.get_mut(old).unwrap();
            *entry = Entry::Rotated;
        }
        self.save(new);
        Ok(previous)
    }

    async fn revoke(&self, hash: &[u8; 32]) -> Result<(), RefreshTokenStoreError> {
        self.entries.lock().unwrap().remove(hash);
        Ok(())
    }
}

#[derive(Debug)]
pub struct PlainHasher;

impl PlainHasher {
    fn plain_hash(password: &SecretSlice<u8>) -> String {
        format!(
            "plain:{}",
            String::from_utf8_lossy(password.expose_secret())
        )
    }
}

#[async_trait]
impl PasswordHasher for PlainHasher {
    async fn hash(&self, password: SecretSlice<u8>) -> Result<String, PasswordError> {
        Ok(Self::plain_hash(&password))
    }

    async fn verify(&self, password: SecretSlice<u8>, hash: String) -> Result<bool, PasswordError> {
        Ok(Self::plain_hash(&password) == hash)
    }

    async fn verify_dummy(&self, _password: SecretSlice<u8>) -> Result<(), PasswordError> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct FakeSigner {
    clock: Arc<ManualClock>,
}

impl FakeSigner {
    pub fn new(clock: Arc<ManualClock>) -> Self {
        Self { clock }
    }

    pub fn token_for(user_id: UserId) -> String {
        format!("access-token-for-{user_id}")
    }

    fn signing_key(&self) -> PublicKey {
        PublicKey {
            kid: SIGNING_KEY_ID,
            key: "test-public-key".into(),
            exp: self.clock.now() + Duration::days(1),
        }
    }
}

#[async_trait]
impl SigningService for FakeSigner {
    async fn sign(&self, sub: Uuid) -> Result<SignedToken, SigningError> {
        Ok(SignedToken {
            token: Self::token_for(UserId(sub)),
            key_id: SIGNING_KEY_ID,
            new_key: None,
        })
    }

    async fn get_verification_keys(&self) -> Result<Vec<PublicKey>, SigningError> {
        Ok(vec![self.signing_key()])
    }
}

#[derive(Debug, Default)]
pub struct InMemoryJwkCache(Mutex<HashSet<Uuid>>);

impl InMemoryJwkCache {
    pub fn with_key(kid: Uuid) -> Self {
        Self(Mutex::new(HashSet::from([kid])))
    }

    pub fn has_key(&self, kid: &Uuid) -> bool {
        self.0.lock().unwrap().contains(kid)
    }
}

#[async_trait]
impl JwkCache for InMemoryJwkCache {
    async fn contains(&self, kid: &Uuid) -> Result<bool, JwkCacheError> {
        Ok(self.has_key(kid))
    }

    async fn insert(&self, key: &JwkRecord) -> Result<(), JwkCacheError> {
        self.0.lock().unwrap().insert(key.kid);
        Ok(())
    }

    async fn insert_many(&self, keys: &[JwkRecord]) -> Result<(), JwkCacheError> {
        self.0.lock().unwrap().extend(keys.iter().map(|k| k.kid));
        Ok(())
    }
}

pub struct TestAuth {
    clock: Arc<ManualClock>,
    users: Arc<InMemoryUsers>,
    jwk_cache: Arc<dyn JwkCache>,
    passwords: Arc<dyn PasswordHasher>,
    signer: Arc<dyn SigningService>,
}

pub struct BuiltAuth {
    pub service: DefaultAuthService,
    pub clock: Arc<ManualClock>,
    pub users: Arc<InMemoryUsers>,
}

impl TestAuth {
    pub fn new() -> Self {
        let clock = Arc::new(ManualClock::new(start_time()));
        Self {
            users: Arc::default(),
            jwk_cache: Arc::new(InMemoryJwkCache::default()),
            passwords: Arc::new(PlainHasher),
            signer: Arc::new(FakeSigner::new(clock.clone())),
            clock,
        }
    }

    pub fn with_passwords(mut self, passwords: impl PasswordHasher) -> Self {
        self.passwords = Arc::new(passwords);
        self
    }

    pub fn with_signer(mut self, signer: impl SigningService) -> Self {
        self.signer = Arc::new(signer);
        self
    }

    pub fn with_jwk_cache(mut self, jwk_cache: Arc<dyn JwkCache>) -> Self {
        self.jwk_cache = jwk_cache;
        self
    }

    pub fn build(self) -> BuiltAuth {
        let service = DefaultAuthService::new(
            self.users.clone(),
            Arc::new(InMemoryFamilies::new(self.clock.clone())),
            self.passwords,
            self.signer,
            self.jwk_cache,
            Arc::new(InMemoryRefreshTokens::new(self.clock.clone())),
            &RefreshTokenSettings::default(),
        )
        .with_clock(self.clock.clone());
        BuiltAuth {
            service,
            clock: self.clock,
            users: self.users,
        }
    }
}
