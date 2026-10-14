use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::refresh_token::RefreshTokenError;

pub struct RefreshToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub exp: DateTime<Utc>,
}

impl RefreshToken {
    pub fn new(user_id: Uuid, exp: DateTime<Utc>) -> Self {
        todo!()
    }

    pub fn encode(&self) -> String {
        todo!()
    }

    pub fn decode_id(token: &str) -> Result<Uuid, RefreshTokenError> {
        todo!()
    }
}
