use std::sync::Arc;

use anyhow::bail;
use arc_swap::ArcSwap;
use uuid::Uuid;

use crate::{cache::JwkRecord, service::PublicKey};

#[derive(Debug, Clone)]
pub struct JwkCache {
    pub cache: Arc<ArcSwap<Vec<JwkRecord>>>,
}

impl JwkCache {
    pub fn new(initial: JwkRecord) -> Self {
        Self {
            cache: Arc::new(ArcSwap::from_pointee(vec![initial])),
        }
    }

    pub fn insert(&self, record: PublicKey) -> anyhow::Result<()> {
        let to_add = JwkRecord::try_from(record)?;
        let cutoff = chrono::Utc::now();
        if to_add.exp <= cutoff {
            bail!("Cannot insert expired JWK record with kid {}", to_add.kid);
        }

        if self.contains(&to_add.kid) {
            bail!("JWK record with kid {} already exists", to_add.kid);
        }

        self.cache.rcu(|current| {
            let live = &current[current.partition_point(|jr| jr.exp <= cutoff)..];
            let mut next = Vec::with_capacity(live.len() + 1);
            next.extend_from_slice(live);
            next.push(to_add.clone());
            next
        });
        Ok(())
    }

    pub fn contains(&self, kid: &Uuid) -> bool {
        self.cache
            .load()
            .binary_search_by(|jr| jr.kid.cmp(kid))
            .is_ok()
    }

    pub fn live(&self) -> Vec<JwkRecord> {
        let now = chrono::Utc::now();
        let keys = self.cache.load();
        let start = keys.partition_point(|jr| jr.exp <= now);
        keys[start..].to_vec()
    }

    pub fn rebuild(&self, records: Vec<PublicKey>) -> anyhow::Result<()> {
        let next = records
            .into_iter()
            .map(JwkRecord::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        self.cache.store(Arc::new(next));
        Ok(())
    }
}
