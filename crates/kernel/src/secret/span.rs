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

/// Whether a segment is one of the two the grammar admits, which is also
/// why a reference needs no escaping: everything outside this alphabet is
/// refused where the reference is built.
fn segment_ok(segment: &str) -> bool {
    !segment.is_empty()
        && segment
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
}

/// The one refusal for text outside the grammar. The action names what
/// was being made of it; the subject is the text itself, so a person sees
/// what was rejected and not only that something was.
fn outside_the_grammar(raw: &str) -> AxError {
    AxError::failure(AxCode::ConfigInvalid, "build a secret reference", raw).with_recovery(
        "write secret:<realm>/<name>, where each segment is non-empty and uses only letters, \
         digits, '-', '_' or '.'",
    )
}

impl SecretRef {
    /// The one place a reference is built from its two segments, and so
    /// the one place the grammar is spelled. Every writer of a vault
    /// place calls this: a caller that assembled `secret:<realm>/<name>`
    /// from a format string could name a place no reader resolves, and
    /// the two spellings would drift the first time either is changed.
    ///
    /// # Errors
    /// `E_CONFIG_INVALID` when a segment is empty or carries a character
    /// outside the reference alphabet.
    pub fn new(realm: &str, name: &str) -> Result<SecretRef, AxError> {
        if !segment_ok(realm) || !segment_ok(name) {
            return Err(outside_the_grammar(&format!("secret:{realm}/{name}")));
        }
        Ok(SecretRef {
            realm: realm.to_owned(),
            name: name.to_owned(),
        })
    }

    /// Reads a reference back out of its text. Text that carries the
    /// prefix and a `/` is handed to [`SecretRef::new`], whose refusal
    /// names this same text: the two halves of the grammar answer with
    /// one error.
    ///
    /// # Errors
    /// `E_CONFIG_INVALID` for text that is not a reference.
    pub fn parse(raw: &str) -> Result<SecretRef, AxError> {
        let Some(rest) = raw.strip_prefix("secret:") else {
            return Err(outside_the_grammar(raw));
        };
        let Some((realm, name)) = rest.split_once('/') else {
            return Err(outside_the_grammar(raw));
        };
        SecretRef::new(realm, name)
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

    /// The builder and the reader are two halves of one grammar: whatever
    /// `new` refuses, `parse` refuses, and the text either accepts is the
    /// same text. The refusal names what was rejected, because a person
    /// holding a 422 has to see which realm or name the city would not
    /// take.
    #[test]
    fn secret_ref_grammar_is_fail_closed() {
        let ok = SecretRef::new("anthropic", "api-key").unwrap();
        assert_eq!(ok.realm(), "anthropic");
        assert_eq!(ok.name(), "api-key");
        assert_eq!(ok.to_string(), "secret:anthropic/api-key");
        assert_eq!(SecretRef::parse("secret:anthropic/api-key").unwrap(), ok);
        for (realm, name) in [
            ("", "key"),
            ("realm", ""),
            ("a b", "key"),
            ("realm", "a/b"),
            ("realm", "key:"),
            ("realm", "key "),
        ] {
            let text = format!("secret:{realm}/{name}");
            let err = SecretRef::new(realm, name).unwrap_err();
            assert_eq!(err.code(), &AxCode::ConfigInvalid, "{text}");
            assert_eq!(err.subject(), text.as_str(), "the refusal names {text}");
        }
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
            assert_eq!(err.subject(), bad, "the refusal names {bad:?}");
        }
    }
}
