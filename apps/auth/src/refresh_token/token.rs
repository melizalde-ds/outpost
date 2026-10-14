use chrono::{DateTime, Utc};
use uuid::Uuid;

pub struct RefreshToken {
    pub hash: [u8; 32],
    pub user_id: Uuid,
    pub exp: DateTime<Utc>,
}

impl RefreshToken {
    pub fn generate(user_id: Uuid, exp: DateTime<Utc>) -> (String, Self) {
        todo!()
    }

    pub fn hash_token(token: &str) -> [u8; 32] {
        todo!()
    }
}
