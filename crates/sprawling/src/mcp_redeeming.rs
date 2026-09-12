// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The name/value pairs a building writes beside an MCP server, with
//! every `secret:realm/name` reference already redeemed.
//!
//! One module for the three transports, because they configure the same
//! thing under two names: a header a hosted server wants and an
//! environment variable a child process wants are both a name, a value,
//! and a value that may be a credential. A second redemption path would
//! be a second answer to "when is plaintext allowed to exist".
//!
//! **Redeemed here rather than at the first call.** A reference the
//! vault does not hold is a configuration mistake, and the person who
//! can act on it is the one editing the file - not a model whose tool
//! stopped answering an hour later.

use std::sync::Arc;

use kernel::{AxCode, AxError, Sealed};

/// One configured pair, ready to be handed to a process or a request.
///
/// Cloning shares a redeemed credential rather than copying it, because
/// one server is one credential however many handles a run holds on it.
#[derive(Clone)]
pub(crate) struct Redeemed {
    name: String,
    value: Value,
}

/// What a configured value turned out to be.
#[derive(Clone)]
enum Value {
    /// Written out in the configuration: an account name, a region, a
    /// fixed tag - anything whose disclosure costs nothing.
    Plain(String),
    /// A `secret:realm/name` reference the vault answered.
    Sealed(Arc<Sealed<String>>),
}

impl Redeemed {
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    /// The plaintext, for the length of the call that writes it out.
    ///
    /// The only face that yields it, so every place a credential can
    /// leave this process is a place that named this method. Named for
    /// what it hands back rather than after `Sealed::expose`, because
    /// this module is where that one call happens and the value travels
    /// onward sealed inside `Redeemed`.
    pub(crate) fn plaintext(&self) -> &str {
        match self.value {
            Value::Plain(ref text) => text,
            Value::Sealed(ref held) => held.expose().as_str(),
        }
    }
}

impl std::fmt::Debug for Redeemed {
    /// Names the pair but never its value: a configured value may be a
    /// redeemed credential, and a `Debug` that printed it would be the
    /// leak [`Sealed`] exists to make unspellable.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Redeemed({})", self.name)
    }
}

/// Redeems every pair a configuration states, in the order it states
/// them.
///
/// # Errors
/// Refuses a pair with no name, and a `secret:realm/name` reference the
/// vault does not answer. A value that is not a reference is carried as
/// written; that is the ordinary case and costs the vault nothing.
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

/// What a refusal may say about a value it is complaining about. A
/// reference names nothing secret and is quoted; anything else is a
/// value this city has no reason to believe is public.
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

    /// A credential never appears in a refusal, and neither does a value
    /// this city has no reason to believe is public.
    #[test]
    fn a_nameless_pair_is_refused_without_quoting_what_it_carried() {
        let pairs = vec![("  ".to_owned(), "hunter2".to_owned())];
        let err = redeem(&pairs, &vault(), "start an mcp server").unwrap_err();
        assert_eq!(err.code(), &AxCode::ConfigInvalid);
        assert!(!err.subject().contains("hunter2"), "{}", err.subject());
    }

    /// A reference the vault does not hold is a configuration mistake,
    /// and it arrives while the file is being read.
    #[test]
    fn a_reference_the_vault_does_not_hold_is_refused_before_anything_starts() {
        let pairs = vec![("API_KEY".to_owned(), "secret:mcp/absent".to_owned())];
        let err = redeem(&pairs, &vault(), "start an mcp server").unwrap_err();
        assert_eq!(err.code(), &AxCode::CredentialMissing);
    }

    /// The one face that yields plaintext is `expose`, and `Debug` is
    /// not it.
    #[test]
    fn debug_names_the_pair_and_never_its_value() {
        let pairs = vec![("API_KEY".to_owned(), "secret:mcp/known".to_owned())];
        let redeemed = redeem(&pairs, &vault(), "start an mcp server").unwrap();
        assert_eq!(format!("{:?}", redeemed[0]), "Redeemed(API_KEY)");
    }
}
