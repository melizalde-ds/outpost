use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Serialize, Serializer};
use serde_json::value::RawValue;
use uuid::Uuid;

use crate::service::PublicKey;

pub(crate) mod jwk;

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
    pub json: Box<RawValue>,
    pub exp: DateTime<Utc>,
}

impl JwkRecord {
    pub fn new(kid: Uuid, data: JwkData, exp: DateTime<Utc>) -> Result<Self> {
        let json = serde_json::value::to_raw_value(&data)
            .with_context(|| format!("failed to serialize JWK {kid}"))?;
        Ok(Self { kid, json, exp })
    }
}

impl Serialize for JwkRecord {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        self.json.serialize(serializer)
    }
}

impl TryFrom<PublicKey> for JwkRecord {
    type Error = anyhow::Error;

    fn try_from(value: PublicKey) -> Result<Self, Self::Error> {
        let data = JwkData::new(value.kid.to_string(), value.key);
        Self::new(value.kid, data, value.exp)
    }
}
