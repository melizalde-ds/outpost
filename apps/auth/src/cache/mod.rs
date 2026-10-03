use std::sync::Arc;

use arc_swap::ArcSwap;
use uuid::Uuid;

mod errors;
mod record;

pub use errors::CacheError;
pub use record::JwkRecord;

#[derive(Debug, Clone)]
pub struct JwkCache {
    pub cache: Arc<ArcSwap<Vec<JwkRecord>>>,
}

impl JwkCache {
    pub fn new(initial: Vec<JwkRecord>) -> Self {
        Self {
            cache: Arc::new(ArcSwap::from_pointee(initial)),
        }
    }

    pub fn insert(&self, to_add: JwkRecord) -> Result<(), CacheError> {
        let cutoff = chrono::Utc::now();
        if to_add.exp <= cutoff {
            return Err(CacheError::ExpiredKey(to_add.kid));
        }

        if self.contains(&to_add.kid) {
            return Ok(());
        }

        self.cache.rcu(|current| {
            let live = &current[current.partition_point(|jr| jr.exp <= cutoff)..];
            live.iter()
                .chain(std::iter::once(&to_add))
                .cloned()
                .collect::<Vec<_>>()
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

    pub fn rebuild(&self, records: Vec<JwkRecord>) {
        self.cache.store(Arc::new(records));
    }
}
