// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::Arc;

use kernel::{AxCode, AxError, Sealed};

#[derive(Clone)]
pub(crate) struct Redeemed {
    name: String,
    value: Value,
}

#[derive(Clone)]
enum Value {

    Plain(String),

    Sealed(Arc<Sealed<String>>),
}

impl Redeemed {
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn plaintext(&self) -> &str {
        match self.value {
            Value::Plain(ref text) => text,
            Value::Sealed(ref held) => held.expose().as_str(),
        }
    }
}

impl std::fmt::Debug for Redeemed {

    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Redeemed({})", self.name)
    }
}

pub(crate) fn redeem(
    pairs: &[(String, String)],
    resolve: &gateway::SecretResolver,
    action: &'static str,
) -> Result<Vec<Redeemed>, AxError> {
    let mut out = Vec::with_capacity(pairs.len());
    for (name, value) in pairs {
        let name = name.trim();
        if name.is_empty() {
            return Err(AxError::failure(
                AxCode::ConfigInvalid,
                action,
                format!("a pair with no name carries {}", masked(value)),
            )
            .with_recovery("give every environment variable and every header a name"));
        }
        let value = value.trim();
        let held = match kernel::SecretRef::parse(value) {
            Ok(reference) => Value::Sealed(Arc::new(resolve(&reference)?)),
            Err(_) => Value::Plain(value.to_owned()),
        };
        out.push(Redeemed {
            name: name.to_owned(),
            value: held,
        });
    }
    Ok(out)
}

fn masked(value: &str) -> String {
    match kernel::SecretRef::parse(value.trim()) {
        Ok(reference) => reference.to_string(),
        Err(_) => "a value".to_owned(),
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

    fn vault() -> gateway::SecretResolver {
        Box::new(|reference: &kernel::SecretRef| {
            if reference.name() == "known" {
                Ok(Sealed::new(Box::new("the key".to_owned())))
            } else {
                Err(AxError::failure(
                    AxCode::CredentialMissing,
                    "resolve a credential",
                    reference.to_string(),
                )
                .with_recovery("store it first"))
            }
        })
    }

    #[test]
    fn a_reference_is_redeemed_and_a_plain_value_is_carried_as_written() {
        let pairs = vec![
            ("API_KEY".to_owned(), "secret:mcp/known".to_owned()),
            ("REGION".to_owned(), " eu ".to_owned()),
        ];
        let redeemed = redeem(&pairs, &vault(), "start an mcp server").unwrap();
        assert_eq!(
            redeemed
                .iter()
                .map(|pair| (pair.name(), pair.plaintext()))
                .collect::<Vec<(&str, &str)>>(),
            vec![("API_KEY", "the key"), ("REGION", "eu")]
        );
    }

    #[test]
    fn a_nameless_pair_is_refused_without_quoting_what_it_carried() {
        let pairs = vec![("  ".to_owned(), "hunter2".to_owned())];
        let err = redeem(&pairs, &vault(), "start an mcp server").unwrap_err();
        assert_eq!(err.code(), &AxCode::ConfigInvalid);
        assert!(!err.subject().contains("hunter2"), "{}", err.subject());
    }

    #[test]
    fn a_reference_the_vault_does_not_hold_is_refused_before_anything_starts() {
        let pairs = vec![("API_KEY".to_owned(), "secret:mcp/absent".to_owned())];
        let err = redeem(&pairs, &vault(), "start an mcp server").unwrap_err();
        assert_eq!(err.code(), &AxCode::CredentialMissing);
    }

    #[test]
    fn debug_names_the_pair_and_never_its_value() {
        let pairs = vec![("API_KEY".to_owned(), "secret:mcp/known".to_owned())];
        let redeemed = redeem(&pairs, &vault(), "start an mcp server").unwrap();
        assert_eq!(format!("{:?}", redeemed[0]), "Redeemed(API_KEY)");
    }
}
