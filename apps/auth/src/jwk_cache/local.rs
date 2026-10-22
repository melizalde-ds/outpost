use std::{collections::BTreeMap, sync::Arc};

use arc_swap::ArcSwapOption;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::jwk_cache::JwkRecord;

/// In-memory copy of the public keys, so they can still be served while the key provider is down.
/// `None` means no full key list has been loaded yet.
#[derive(Debug, Default)]
pub struct LocalJwkSet(ArcSwapOption<BTreeMap<Uuid, JwkRecord>>);

impl LocalJwkSet {
    /// Adds one key, but only after a full load: on its own it would hide the other live keys.
    pub fn insert(&self, record: JwkRecord) {
        self.0.rcu(|keys| {
            keys.as_ref().map(|keys| {
                let mut keys = BTreeMap::clone(keys);
                keys.insert(record.kid, record.clone());
                Arc::new(keys)
            })
        });
    }

    pub fn replace(&self, records: Vec<JwkRecord>) {
        let keys = records.into_iter().map(|r| (r.kid, r)).collect();
        self.0.store(Some(Arc::new(keys)));
    }

    pub fn live(&self, now: DateTime<Utc>) -> Option<Vec<JwkRecord>> {
        self.0
            .load()
            .as_ref()
            .map(|keys| keys.values().filter(|key| key.exp > now).cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone};
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    use super::*;
    use crate::jwk_cache::record::JwkData;

    const KEY_A: Uuid = Uuid::from_u128(0xa);
    const KEY_B: Uuid = Uuid::from_u128(0xb);

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
    }

    fn record(kid: Uuid, exp: DateTime<Utc>) -> JwkRecord {
        JwkRecord::new(kid, JwkData::new(kid.to_string(), "x"), exp).unwrap()
    }

    fn live_record(kid: Uuid) -> JwkRecord {
        record(kid, now() + Duration::days(1))
    }

    fn kids(keys: Option<Vec<JwkRecord>>) -> Option<Vec<Uuid>> {
        keys.map(|keys| keys.into_iter().map(|k| k.kid).collect())
    }

    #[test]
    fn live_before_first_load_returns_none() {
        // Arrange
        let keys = LocalJwkSet::default();

        // Act
        let live = keys.live(now());

        // Assert
        assert!(live.is_none());
    }

    #[test]
    fn live_after_empty_load_returns_no_keys() {
        // Arrange
        let keys = LocalJwkSet::default();
        keys.replace(vec![]);

        // Act
        let live = kids(keys.live(now()));

        // Assert
        assert_eq!(live, Some(vec![]));
    }

    #[rstest]
    #[case::expired(-1, false)]
    #[case::expiring_now(0, false)]
    #[case::not_yet_expired(1, true)]
    fn live_key_near_expiry_is_kept_only_before_exp(
        #[case] exp_offset_secs: i64,
        #[case] kept: bool,
    ) {
        // Arrange
        let keys = LocalJwkSet::default();
        keys.replace(vec![record(
            KEY_A,
            now() + Duration::seconds(exp_offset_secs),
        )]);

        // Act
        let live = kids(keys.live(now())).unwrap();

        // Assert
        assert_eq!(live.contains(&KEY_A), kept);
    }

    #[test]
    fn insert_before_first_load_is_ignored() {
        // Arrange
        let keys = LocalJwkSet::default();

        // Act
        keys.insert(live_record(KEY_A));

        // Assert
        assert!(keys.live(now()).is_none());
    }

    #[test]
    fn insert_after_load_adds_the_key() {
        // Arrange
        let keys = LocalJwkSet::default();
        keys.replace(vec![live_record(KEY_A)]);

        // Act
        keys.insert(live_record(KEY_B));

        // Assert
        assert_eq!(kids(keys.live(now())), Some(vec![KEY_A, KEY_B]));
    }

    #[test]
    fn replace_drops_keys_missing_from_the_new_list() {
        // Arrange
        let keys = LocalJwkSet::default();
        keys.replace(vec![live_record(KEY_A)]);

        // Act
        keys.replace(vec![live_record(KEY_B)]);

        // Assert
        assert_eq!(kids(keys.live(now())), Some(vec![KEY_B]));
    }
}
