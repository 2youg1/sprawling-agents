// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Whose identity this city can call which model under: the part of the
//! worker's state that `endpoint_attached`, `model_selected`,
//! `endpoint_lost` and `secret_captured` change (sprawling-SPEC.md 8-90).

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use kernel::{AxError, EventKind, Payload};

use super::subscription::Expiries;

/// The credentials the worker holds, one value because one family of
/// records changes them and one question reads them.
pub(in crate::assembly) struct Credentials {
    /// Every endpoint the person attached and every model they chose,
    /// folded from the ledger. The worker keeps its own copy because a
    /// dispatch needs it synchronously, before the record it just wrote
    /// has reached any observer.
    pub(in crate::assembly) book: gateway::EndpointBook,
    /// The vault. Shared because a redemption closure outlives the call
    /// that builds it; the lock is held for one resolve at a time.
    pub(in crate::assembly) vault: Arc<Mutex<gateway::Custodian>>,
    /// When each subscription credential stops working, by provider.
    /// Folded from the capture records, so a restarted city renews on
    /// the same schedule rather than discovering expiry through a 401.
    pub(in crate::assembly) expiries: Expiries,
    /// Logins begun and not yet redeemed, by provider. Held in memory
    /// on purpose: a PKCE verifier proves that the process which asked
    /// is the process which redeems, so a verifier that outlived the
    /// process would be proving nothing. A restart means starting the
    /// login again, which is one browser visit.
    pub(in crate::assembly) logins: BTreeMap<String, gateway::OauthPending>,
}

impl Credentials {
    /// The credentials a worker opens with: the book and expiries its
    /// history folded to, the vault it was handed, and no login begun.
    pub(in crate::assembly) fn opened(
        book: gateway::EndpointBook,
        expiries: Expiries,
        vault: gateway::Custodian,
    ) -> Credentials {
        Credentials {
            book,
            vault: Arc::new(Mutex::new(vault)),
            expiries,
            logins: BTreeMap::new(),
        }
    }

    /// Folds one line the worker wrote, whoever it was written for.
    ///
    /// # Errors
    /// A credential record this build cannot read: the book would then
    /// state something the history does not.
    pub(in crate::assembly) fn absorb(
        &mut self,
        kind: EventKind,
        data: &Payload,
    ) -> Result<(), AxError> {
        self.expiries.absorb(kind, data)?;
        self.book.apply_payload(kind, data)
    }
}
