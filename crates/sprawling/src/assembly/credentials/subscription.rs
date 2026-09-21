// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a subscription credential is called, and when it stops working.
//!
//! The two belong together: the expiry table is keyed by provider, and
//! the only place a provider name and an expiry appear side by side is
//! the `secret_captured` record, whose `ref` field spells the
//! credential. One module writes that spelling and one module reads it
//! back, so a rename cannot leave the reader looking for the old name.
//!
//! The table is folded from the history like every other book the
//! worker keeps. It used to be written only by the process that logged
//! in, so a restarted city had no expiry for anything and renewed
//! nothing — and discovered each expiry as a 401 in the middle of a
//! run, losing that turn (sprawling-SPEC.md 8-11).

use kernel::{AxError, EventKind, Payload, SecretRef};

/// Where a provider's access token lives.
///
/// # Errors
/// Propagates a provider name that is not a legal secret realm.
pub(in crate::assembly) fn oauth_ref(provider: &str) -> Result<SecretRef, AxError> {
    SecretRef::parse(&format!("secret:{provider}/{OAUTH}"))
}

/// Where a provider's refresh token lives.
///
/// # Errors
/// Propagates a provider name that is not a legal secret realm.
pub(in crate::assembly) fn oauth_refresh_ref(provider: &str) -> Result<SecretRef, AxError> {
    SecretRef::parse(&format!("secret:{provider}/{OAUTH}-refresh"))
}

/// The provider an access token belongs to, read back from the
/// spelling above. A reference to anything else answers `None`.
fn provider_of(reference: &str) -> Option<&str> {
    reference
        .strip_prefix("secret:")?
        .strip_suffix(&format!("/{OAUTH}"))
}

/// The name of an access token within its provider's realm.
const OAUTH: &str = "oauth";

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
        let Some(provider) = provider_of(reference) else {
            return;
        };
        let Some(at) = data.get("expires_at").and_then(serde_json::Value::as_u64) else {
            return;
        };
        self.by_provider.insert(provider.to_owned(), at);
    }

    /// When this provider's credential stops working, if the provider
    /// ever said.
    pub(in crate::assembly) fn of(&self, provider: &str) -> Option<u64> {
        self.by_provider.get(provider).copied()
    }
}
