// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a subscription credential is called, and when it stops working.
//!
//! The two belong together: the expiry table is keyed by provider, and
//! the only place a provider name and an expiry appear side by side is
//! the `secret_captured` record, whose `ref` field names the credential.
//! The places a subscription token occupies are stated here and nowhere
//! else, and [`kernel::SecretRef`] is what turns a place into text, so a
//! rename cannot leave the reader looking for the old name.
//!
//! The table is folded from the history like every other book the
//! worker keeps. It used to be written only by the process that logged
//! in, so a restarted city had no expiry for anything and renewed
//! nothing — and discovered each expiry as a 401 in the middle of a
//! run, losing that turn (sprawling-SPEC.md 8-11).

use kernel::{AxError, EventKind, Payload, SecretRef};

/// The name of an access token within its provider's realm.
const OAUTH: &str = "oauth";

/// Where a provider's refresh token lives. A separate name from the
/// access token's because a provider issues two credentials and either
/// one can be replaced without the other.
const OAUTH_REFRESH: &str = "oauth-refresh";

/// Where a provider's access token lives.
///
/// # Errors
/// Propagates a provider name that is not a legal secret realm.
pub(in crate::assembly) fn oauth_ref(provider: &str) -> Result<SecretRef, AxError> {
    SecretRef::new(provider, OAUTH)
}

/// Where a provider's refresh token lives.
///
/// # Errors
/// Propagates a provider name that is not a legal secret realm.
pub(in crate::assembly) fn oauth_refresh_ref(provider: &str) -> Result<SecretRef, AxError> {
    SecretRef::new(provider, OAUTH_REFRESH)
}

/// The provider an access token belongs to, read back from the record
/// that named it.
///
/// A reference to anything else — another realm's key, or this
/// provider's refresh token — answers `None`: this table holds access
/// tokens, and the reader asks the grammar rather than trimming text.
fn access_provider(reference: &str) -> Option<String> {
    let place = match SecretRef::parse(reference) {
        Ok(place) => place,
        // A `ref` outside the grammar names no vault place, so it names
        // no provider's token either, and this book has nothing to fold
        // in from it.
        Err(_) => return None,
    };
    if place.name() != OAUTH {
        return None;
    }
    Some(place.realm().to_owned())
}

/// When each provider's subscription credential stops working, in the
/// city's own clock.
///
/// Folded from `secret_captured`, which is where the provider stated
/// it. Nothing here is a secret: an expiry is a time, and the token it
/// describes is in the vault.
#[derive(Default)]
pub(in crate::assembly) struct Expiries {
    by_provider: std::collections::BTreeMap<String, u64>,
}

impl Expiries {
    /// Folds one line in, from a history being replayed or from the
    /// running city that just wrote it.
    ///
    /// A capture with no stated expiry leaves the table alone: not
    /// knowing when something expires is not a reason to renew it every
    /// time, and it is not a reason to forget an expiry either.
    pub(in crate::assembly) fn absorb(&mut self, kind: EventKind, payload: &Payload) {
        if kind != EventKind::SecretCaptured {
            return;
        }
        let data = payload.as_map();
        let Some(reference) = data.get("ref").and_then(serde_json::Value::as_str) else {
            return;
        };
        let Some(provider) = access_provider(reference) else {
            return;
        };
        let Some(at) = data.get("expires_at").and_then(serde_json::Value::as_u64) else {
            return;
        };
        self.by_provider.insert(provider, at);
    }

    /// When this provider's credential stops working, if the provider
    /// ever said.
    pub(in crate::assembly) fn of(&self, provider: &str) -> Option<u64> {
        self.by_provider.get(provider).copied()
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test code"
)]
mod tests {
    use super::*;

    /// One `secret_captured` line, as the ledger carries it.
    fn capture(reference: &str, expires_at: Option<u64>) -> Payload {
        let mut map = serde_json::Map::new();
        map.insert(
            "ref".to_owned(),
            serde_json::Value::String(reference.to_owned()),
        );
        if let Some(at) = expires_at {
            map.insert(
                "expires_at".to_owned(),
                serde_json::Value::Number(at.into()),
            );
        }
        Payload::new(map).expect("a map of a string and a number is a payload")
    }

    /// The table keys on the access token's place and on nothing else.
    /// A refresh token's capture, another realm's key, and a `ref`
    /// outside the grammar all leave it alone: renewal asks the access
    /// token's expiry, and a reader that guessed would renew the wrong
    /// credential or leave a dead one alone.
    #[test]
    fn only_an_access_token_capture_enters_the_expiry_table() {
        let mut expiries = Expiries::default();
        for (reference, at) in [
            ("secret:anthropic/oauth-refresh", 1_000),
            ("secret:anthropic/api-key", 2_000),
            ("secret:openai/oauth-refresh", 3_000),
            ("anthropic/oauth", 4_000),
            ("secret:two words/oauth", 5_000),
        ] {
            expiries.absorb(EventKind::SecretCaptured, &capture(reference, Some(at)));
        }
        assert_eq!(
            expiries.of("anthropic"),
            None,
            "a refresh token is not an access token"
        );
        assert_eq!(expiries.of("openai"), None);
        assert_eq!(
            expiries.of("two words"),
            None,
            "a realm the grammar refuses"
        );
        for (reference, at) in [
            ("secret:anthropic/oauth", 6_000),
            ("secret:openai/oauth", 7_000),
        ] {
            expiries.absorb(EventKind::SecretCaptured, &capture(reference, Some(at)));
        }
        assert_eq!(expiries.of("anthropic"), Some(6_000));
        assert_eq!(expiries.of("openai"), Some(7_000));
    }

    /// A capture with no expiry leaves a stated one in place rather than
    /// clearing it: not knowing when something stops working is not a
    /// reason to renew it on every call.
    #[test]
    fn a_capture_without_an_expiry_keeps_the_one_already_known() {
        let mut expiries = Expiries::default();
        expiries.absorb(
            EventKind::SecretCaptured,
            &capture("secret:anthropic/oauth", Some(1_000)),
        );
        expiries.absorb(
            EventKind::SecretCaptured,
            &capture("secret:anthropic/oauth", None),
        );
        assert_eq!(expiries.of("anthropic"), Some(1_000));
    }
}
