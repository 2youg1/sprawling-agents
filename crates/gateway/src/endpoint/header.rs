// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The value of one header a person added to an endpoint.
//!
//! Two documents used to disagree about what such a value may hold:
//! the registration said it could be a `secret:realm/name` reference,
//! the endpoint configuration said it was never a credential, and no
//! reader redeemed anything — so a person writing a reference got a
//! literal header and a 401, and a person writing the key itself got
//! the key in the ledger. The value is a type now, and the type
//! decides: a reference is redeemed in the slot before the wire, and a
//! literal that reads as a credential is refused where it is entered.

use kernel::{AxCode, AxError, SecretRef};

/// What one extra header carries to the provider.
///
/// Exhaustive and closed: a third kind of value would have to state how
/// it reaches the wire, which is the question this type exists to
/// answer once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeaderValue {
    /// A literal, sent verbatim. It has passed
    /// [`kernel::secret::scan`], so it is text a person meant to be
    /// read — a tenant name, an API version, a routing tag.
    Plain(String),
    /// A credential in the vault. It is redeemed where the request is
    /// written and nowhere earlier, so the ledger records the reference
    /// and the process holds the plaintext for one statement.
    Redeemed(SecretRef),
}

impl HeaderValue {
    /// Reads the text a person entered for one header.
    ///
    /// **One rule, stated once.** Text this city can read as a vault
    /// reference is that reference; every other text is a literal.
    /// A literal is then scanned, and a scan hit is refused here rather
    /// than at the wire: the refusal has to reach the person while they
    /// are still looking at the box they typed it into, and every path
    /// onward from here writes the value into the ledger.
    ///
    /// A text that was meant as a reference but is misspelled reads as
    /// a literal, because the reference grammar has one reader and it
    /// is [`SecretRef::parse`]; the header then reaches the provider
    /// as typed and the provider answers 401, which is the shortest
    /// path this side can offer to "that is not a reference".
    ///
    /// # Errors
    /// `E_CONFIG_INVALID` when the literal reads as a credential.
    pub fn parse(name: &str, text: &str) -> Result<HeaderValue, AxError> {
        if let Ok(reference) = SecretRef::parse(text) {
            return Ok(HeaderValue::Redeemed(reference));
        }
        if kernel::secret::scan(text.as_bytes()).is_empty() {
            return Ok(HeaderValue::Plain(text.to_owned()));
        }
        Err(AxError::failure(
            AxCode::ConfigInvalid,
            "set an extra header on an endpoint",
            format!("the value of `{name}` reads as a credential"),
        )
        .with_recovery(
            "keep the credential in the vault and write `secret:<realm>/<name>` in this \
             box: the header then carries the key and the record carries the reference",
        ))
    }

    /// How a record spells this value, which is what [`HeaderValue::parse`]
    /// reads back.
    #[must_use]
    pub fn spelled(&self) -> String {
        match self {
            HeaderValue::Plain(text) => text.clone(),
            HeaderValue::Redeemed(reference) => reference.to_string(),
        }
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

    #[test]
    fn a_reference_is_redeemed_and_a_word_is_sent_as_it_stands() {
        assert_eq!(
            HeaderValue::parse("x-tenant", "east").unwrap(),
            HeaderValue::Plain("east".to_owned())
        );
        let held = HeaderValue::parse("x-key", "secret:openai/api").unwrap();
        assert_eq!(
            held,
            HeaderValue::Redeemed(SecretRef::parse("secret:openai/api").unwrap())
        );
        // Both arms round-trip through the record they are written into.
        for value in [held, HeaderValue::Plain("2023-06-01".to_owned())] {
            assert_eq!(
                HeaderValue::parse("x-any", &value.spelled()).unwrap(),
                value
            );
        }
    }

    /// The key itself never becomes a ledger entry: the refusal names
    /// the box and the way out, and never repeats the value.
    #[test]
    fn a_key_typed_into_a_header_box_is_refused_where_it_is_typed() {
        // Assembled rather than written whole, so this repository's own
        // secret scanner does not read the fixture as a leaked key.
        let key = format!(
            "sk-{}-{}{}{}",
            "ant-api03", "Zx8Qv2LmNpR4", "tYw7BcDfGhJk", "LmNpQrStUvWxYz01"
        );
        let refusal = HeaderValue::parse("x-api-key", &key).unwrap_err();
        assert_eq!(*refusal.code(), AxCode::ConfigInvalid);
        assert!(!refusal.subject().contains(&key));
        assert!(!refusal.recovery().contains(&key));
        assert!(refusal.recovery().contains("secret:"));
    }
}
