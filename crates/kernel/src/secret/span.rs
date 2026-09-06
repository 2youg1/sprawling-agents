// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Secret references and spans: the shapes that name secrets.

use crate::error::{AxCode, AxError};

/// `secret:<realm>/<name>`. Deliberately a separate parser from Locator:
/// sharing the grammar would mean a read path could resolve a secret
/// (7.1's type-level answer). Malformed shapes are `E_CONFIG_INVALID`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SecretRef {
    realm: String,
    name: String,
}

impl SecretRef {
    pub fn parse(raw: &str) -> Result<SecretRef, AxError> {
        let malformed = || {
            AxError::failure(AxCode::ConfigInvalid, "parse secret reference", raw)
                .with_recovery("write secret:<realm>/<name>, both segments non-empty")
        };
        let rest = raw.strip_prefix("secret:").ok_or_else(malformed)?;
        let (realm, name) = rest.split_once('/').ok_or_else(malformed)?;
        let segment_ok = |s: &str| {
            !s.is_empty()
                && s.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        };
        if !segment_ok(realm) || !segment_ok(name) || name.contains('/') {
            return Err(malformed());
        }
        Ok(SecretRef {
            realm: realm.to_owned(),
            name: name.to_owned(),
        })
    }

    pub fn realm(&self) -> &str {
        &self.realm
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl std::fmt::Display for SecretRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "secret:{}/{}", self.realm, self.name)
    }
}

impl serde::Serialize for SecretRef {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> serde::Deserialize<'de> for SecretRef {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        SecretRef::parse(&raw).map_err(serde::de::Error::custom)
    }
}

/// One detected span. `provider` names the shape-table entry; `None` is
/// an entropy hit. The span carries offsets only — never the bytes (a
/// finding that quoted its match would itself be a leak surface).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretSpan {
    pub start: usize,
    pub len: usize,
    pub provider: Option<&'static str>,
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
    #[test]
    fn secret_ref_grammar_is_fail_closed() {
        let ok = SecretRef::parse("secret:anthropic/api-key").unwrap();
        assert_eq!(ok.realm(), "anthropic");
        assert_eq!(ok.name(), "api-key");
        assert_eq!(ok.to_string(), "secret:anthropic/api-key");
        for bad in [
            "secret:",
            "secret:/x",
            "secret:a/",
            "secret:a b/c",
            "cas:b3-ab",
            "secret:a/b/c",
            "SECRET:a/b",
        ] {
            let err = SecretRef::parse(bad).unwrap_err();
            assert_eq!(err.code(), &AxCode::ConfigInvalid, "{bad:?}");
        }
    }
}
