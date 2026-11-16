// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError, SecretRef};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum HeaderValue {

    Plain(String),

    Redeemed(SecretRef),
}

impl HeaderValue {

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

        for value in [held, HeaderValue::Plain("2023-06-01".to_owned())] {
            assert_eq!(
                HeaderValue::parse("x-any", &value.spelled()).unwrap(),
                value
            );
        }
    }

    #[test]
    fn a_key_typed_into_a_header_box_is_refused_where_it_is_typed() {

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
