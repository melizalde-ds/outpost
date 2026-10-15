use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Serialize, Serializer};
use serde_json::value::RawValue;
use uuid::Uuid;

use crate::{jwk_cache::JwkRecordError, signing::PublicKey};

#[derive(Debug, Clone, Serialize)]
pub struct JwkData {
    kty: &'static str,
    crv: &'static str,
    kid: String,
    alg: &'static str,
    #[serde(rename = "use")]
    use_: &'static str,
    x: String,
}

impl JwkData {
    pub fn new(kid: impl Into<String>, x: impl Into<String>) -> Self {
        Self {
            kty: "OKP",
            crv: "Ed25519",
            kid: kid.into(),
            alg: "EdDSA",
            use_: "sig",
            x: x.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct JwkRecord {
    pub kid: Uuid,
    pub json: Arc<RawValue>,
    pub exp: DateTime<Utc>,
}

impl JwkRecord {
    pub fn new(kid: Uuid, data: JwkData, exp: DateTime<Utc>) -> Result<Self, JwkRecordError> {
        let box_json = serde_json::value::to_raw_value(&data)?;
        let json = Arc::from(box_json);
        Ok(Self { kid, json, exp })
    }
}

impl Serialize for JwkRecord {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.json.serialize(serializer)
    }
}

impl TryFrom<PublicKey> for JwkRecord {
    type Error = JwkRecordError;

    fn try_from(value: PublicKey) -> Result<Self, Self::Error> {
        let data = JwkData::new(value.kid.to_string(), value.key);
        Self::new(value.kid, data, value.exp)
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use pretty_assertions::assert_eq;
    use serde_json::json;

    use super::*;

    #[test]
    fn try_from_public_key_serializes_as_ed25519_jwk() {
        // Arrange
        let key = PublicKey {
            kid: Uuid::from_u128(1),
            key: "public-key-bytes".into(),
            exp: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        };

        // Act
        let record = JwkRecord::try_from(key).unwrap();

        // Assert
        assert_eq!(
            serde_json::to_value(&record).unwrap(),
            json!({
                "kty": "OKP",
                "crv": "Ed25519",
                "kid": "00000000-0000-0000-0000-000000000001",
                "alg": "EdDSA",
                "use": "sig",
                "x": "public-key-bytes",
            })
        );
    }
}
