// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::event::{RunId, Seq};

pub const IDEM_DERIVE_V: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IdemKey {
    v: u8,
    digest: [u8; 16],
}

impl IdemKey {

    pub fn derive(run: &RunId, seq: Seq, action_canonical: &[u8]) -> IdemKey {
        let mut hasher = blake3::Hasher::new();
        hasher.update(run.as_bytes());
        hasher.update(&seq.value().to_le_bytes());
        hasher.update(action_canonical);
        let mut digest = [0u8; 16];
        hasher.finalize_xof().fill(&mut digest);
        IdemKey {
            v: IDEM_DERIVE_V,
            digest,
        }
    }

    fn parse(raw: &str) -> Option<IdemKey> {
        let rest = raw.strip_prefix("idem")?;
        let (v_raw, hex) = rest.split_once('-')?;
        if v_raw != "1" {
            return None;
        }
        let digest = crate::locator::decode_hex_fixed::<16>(hex)?;
        Some(IdemKey {
            v: IDEM_DERIVE_V,
            digest,
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct IdemGuard {
    key: IdemKey,
}

impl IdemGuard {

    #[must_use]
    pub fn key(&self) -> &IdemKey {
        &self.key
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Duplicate {
    key: IdemKey,
}

impl Duplicate {
    #[must_use]
    pub fn key(&self) -> &IdemKey {
        &self.key
    }
}

pub fn claim(seen: &mut BTreeSet<IdemKey>, key: IdemKey) -> Result<IdemGuard, Duplicate> {
    if seen.contains(&key) {
        return Err(Duplicate { key });
    }
    seen.insert(key);
    Ok(IdemGuard { key })
}

impl std::fmt::Display for IdemKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "idem{}-", self.v)?;
        crate::locator::write_hex(f, &self.digest)
    }
}

impl Serialize for IdemKey {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for IdemKey {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        IdemKey::parse(&raw).ok_or_else(|| {
            serde::de::Error::custom("expected `idem1-` plus 32 lowercase hex digits")
        })
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;
    use crate::event::{RunId, Seq};

    fn run() -> RunId {
        RunId::from_bytes([7; 16])
    }

    #[test]
    fn same_inputs_same_key_always() {
        let a = IdemKey::derive(&run(), Seq::new(42), b"tool:exec{cmd}");
        let b = IdemKey::derive(&run(), Seq::new(42), b"tool:exec{cmd}");
        assert_eq!(a, b);
    }

    #[test]
    fn any_input_change_changes_the_key() {
        let base = IdemKey::derive(&run(), Seq::new(42), b"action");
        assert_ne!(base, IdemKey::derive(&run(), Seq::new(43), b"action"));
        assert_ne!(
            base,
            IdemKey::derive(&RunId::from_bytes([8; 16]), Seq::new(42), b"action")
        );
        assert_ne!(base, IdemKey::derive(&run(), Seq::new(42), b"actioN"));
    }

    #[test]
    fn display_carries_the_derivation_version() {
        let key = IdemKey::derive(&run(), Seq::FIRST, b"x");
        let text = key.to_string();
        assert!(text.starts_with("idem1-"), "{text}");
        assert_eq!(text.len(), "idem1-".len() + 32, "16 bytes as 32 hex digits");
    }

    #[test]
    fn serde_roundtrips_the_string_form() {
        let key = IdemKey::derive(&run(), Seq::new(5), b"payload");
        let json = serde_json::to_string(&key).unwrap();
        let back: IdemKey = serde_json::from_str(&json).unwrap();
        assert_eq!(back, key);
        assert!(serde_json::from_str::<IdemKey>("\"idem1-zz\"").is_err());
        assert!(
            serde_json::from_str::<IdemKey>("\"idem9-00000000000000000000000000000000\"").is_err()
        );
    }

    #[test]
    fn a_key_is_claimable_once_and_the_second_claim_names_it() {
        let key = IdemKey::derive(&run(), Seq::FIRST, b"send mail");
        let other = IdemKey::derive(&run(), Seq::FIRST, b"send other mail");
        let mut seen = BTreeSet::new();
        let guard = claim(&mut seen, key).expect("the first claim on a fresh key");
        assert_eq!(guard.key(), &key);
        let again = claim(&mut seen, key).expect_err("the second claim is a duplicate");
        assert_eq!(again.key(), &key);
        assert!(
            claim(&mut seen, other).is_ok(),
            "a different action is a different key"
        );
    }

    proptest::proptest! {
        #[test]
        fn rederivation_is_identity(
            run_bytes in proptest::array::uniform16(0u8..=255),
            seq in 0u64..,
            action in proptest::collection::vec(0u8..=255, 0..64),
        ) {
            let r = RunId::from_bytes(run_bytes);
            let first = IdemKey::derive(&r, Seq::new(seq), &action);
            let second = IdemKey::derive(&r, Seq::new(seq), &action);
            proptest::prop_assert_eq!(first, second);
        }
    }
}
