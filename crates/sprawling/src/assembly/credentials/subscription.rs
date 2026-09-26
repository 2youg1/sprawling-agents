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

use kernel::event::record::SecretCaptured;
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

/// How a credential travels to an endpoint on one face
/// (sprawling-SPEC.md 8-81).
///
/// A subscription's access token is a bearer token on every face: the
/// vendor answers 401 when it arrives as the messages face's
/// `x-api-key`. Any other reference travels as the face says. A header
/// the person named wins over both.
pub(in crate::assembly) fn auth_for(
    dialect: kernel::DialectKind,
    reference: SecretRef,
    header: Option<String>,
) -> gateway::AuthSpec {
    match (header, reference.name() == OAUTH) {
        (None, true) => gateway::AuthSpec::Bearer(reference),
        (header, _) => gateway::AuthSpec::for_dialect(dialect, reference, header),
    }
}

/// The provider an access token belongs to, read back from the record
/// that named it.
///
/// A reference to anything else — another realm's key, or this
/// provider's refresh token — answers `None`: this table holds access
/// tokens, and the reader asks the grammar rather than trimming text.
fn access_provider(place: &SecretRef) -> Option<String> {
    (place.name() == OAUTH).then(|| place.realm().to_owned())
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
    ///
    /// # Errors
    /// A `secret_captured` line this build cannot read, a `ref` outside
    /// the reference grammar among them: a fold that skipped it would
    /// renew nothing after a restart, and nothing would say why.
    pub(in crate::assembly) fn absorb(
        &mut self,
        kind: EventKind,
        payload: &Payload,
    ) -> Result<(), AxError> {
        if kind != EventKind::SecretCaptured {
            return Ok(());
        }
        let captured: SecretCaptured = payload.read()?;
        if let (Some(provider), Some(at)) =
            (access_provider(&captured.reference), captured.expires_at)
        {
            self.by_provider.insert(provider, at);
        }
        Ok(())
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
    /// A refresh token's capture and another realm's key leave it alone,
    /// and a `ref` outside the grammar is refused: renewal asks the
    /// access token's expiry, and a reader that guessed would renew the
    /// wrong credential or leave a dead one alone.
    #[test]
    fn only_an_access_token_capture_enters_the_expiry_table() {
        let mut expiries = Expiries::default();
        for (reference, at) in [
            ("secret:anthropic/oauth-refresh", 1_000),
            ("secret:anthropic/api-key", 2_000),
            ("secret:openai/oauth-refresh", 3_000),
        ] {
            expiries
                .absorb(EventKind::SecretCaptured, &capture(reference, Some(at)))
                .unwrap();
        }
        for (reference, at) in [
            ("anthropic/oauth", 4_000),
            ("secret:two words/oauth", 5_000),
        ] {
            assert!(
                expiries
                    .absorb(EventKind::SecretCaptured, &capture(reference, Some(at)))
                    .is_err()
            );
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
            expiries
                .absorb(EventKind::SecretCaptured, &capture(reference, Some(at)))
                .unwrap();
        }
        assert_eq!(expiries.of("anthropic"), Some(6_000));
        assert_eq!(expiries.of("openai"), Some(7_000));
    }

    /// A capture this build cannot read is refused rather than skipped:
    /// a fold that dropped it would renew nothing after a restart and
    /// meet the expiry as a 401 mid-run, with no line saying why.
    #[test]
    fn a_capture_that_will_not_read_is_refused_rather_than_skipped() {
        let mut map = capture("secret:anthropic/oauth", None).as_map().clone();
        map.insert(
            "expires_at".to_owned(),
            serde_json::Value::String("soon".to_owned()),
        );
        let mut expiries = Expiries::default();
        let said = format!(
            "{:?}",
            expiries.absorb(EventKind::SecretCaptured, &Payload::new(map).unwrap())
        );
        assert!(said.starts_with("Err("), "{said}");
    }

    /// A capture with no expiry leaves a stated one in place rather than
    /// clearing it: not knowing when something stops working is not a
    /// reason to renew it on every call.
    #[test]
    fn a_capture_without_an_expiry_keeps_the_one_already_known() {
        let mut expiries = Expiries::default();
        expiries
            .absorb(
                EventKind::SecretCaptured,
                &capture("secret:anthropic/oauth", Some(1_000)),
            )
            .unwrap();
        expiries
            .absorb(
                EventKind::SecretCaptured,
                &capture("secret:anthropic/oauth", None),
            )
            .unwrap();
        assert_eq!(expiries.of("anthropic"), Some(1_000));
    }
}
