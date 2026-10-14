use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::repository::{TokenFamilyId, UserId};

pub struct RefreshToken {
    pub user_id: UserId,
    pub hash: [u8; 32],
    pub exp: DateTime<Utc>,
    pub family_exp: DateTime<Utc>,
}

impl RefreshToken {
    pub fn generate(
        user_id: UserId,
        family_id: TokenFamilyId,
        exp: DateTime<Utc>,
        family_exp: DateTime<Utc>,
    ) -> Result<(String, Self), getrandom::Error> {
        let mut secret = [0u8; 32];
        getrandom::fill(&mut secret)?;
        let token = format!(
            "{}.{}",
            family_id.0.simple(),
            URL_SAFE_NO_PAD.encode(secret)
        );
        let hash = hash_token(&token);

        Ok((
            token,
            Self {
                user_id,
                hash,
                exp,
                family_exp,
            },
        ))
    }

    pub fn parse(token: &str) -> Option<(TokenFamilyId, [u8; 32])> {
        let (family_id, secret) = token.split_once('.')?;
        let family_id = Uuid::parse_str(family_id).ok()?;
        let secret = URL_SAFE_NO_PAD.decode(secret).ok()?;
        if secret.len() != 32 {
            return None;
        }

        Some((family_id.into(), hash_token(token)))
    }
}

fn hash_token(token: &str) -> [u8; 32] {
    Sha256::digest(token.as_bytes()).into()
}
