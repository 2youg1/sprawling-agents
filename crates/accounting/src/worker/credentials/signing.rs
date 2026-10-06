// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The vault: the one way a credential enters, and the closure that
//! redeems a reference.
//!
//! No login lives here. Subscription quota enters the city through the
//! vendor's own harness, where the person signs in (`crates/gateway/Spec.lean`
//! §8-5), so the only credential this city takes is a key a person
//! hands it.

use std::sync::Arc;

use kernel::event::record::SecretCaptured;
use kernel::{AxError, EventKind, Payload};

use crate::held_vault::{poisoned_vault, resolving};

use super::super::RunWorker;

/// How a credential reached the vault, as its `secret_captured` record
/// states it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::worker) enum Arrival {
    /// A person enrolled it on the host, under a name they chose.
    Enrolment,
    /// It was pasted into a dispatch and taken into custody there
    /// (`crates/sprawling/Spec.lean` §8-87).
    Pasted,
}

impl Arrival {
    fn spelling(self) -> &'static str {
        match self {
            Arrival::Enrolment => "enrolment",
            Arrival::Pasted => "pasted",
        }
    }
}

impl RunWorker {
    /// Puts one credential in the vault. Nothing about it reaches the
    /// ledger but the fact that it happened, and how it arrived.
    ///
    /// The vault key and the `ref` the record states are one value built
    /// once here: a route that announced a reference it spelled itself
    /// would answer with a place the vault may never have been told.
    pub(in crate::worker) fn put_secret(
        &mut self,
        reference: &kernel::SecretRef,
        value: kernel::Sealed<String>,
        arrival: Arrival,
    ) -> Result<(), AxError> {
        {
            let mut vault = self
                .credentials
                .vault
                .lock()
                .map_err(|_| poisoned_vault())?;
            vault.set(reference, value.into_vault_value())?;
        }
        // Before the record, so a refused append still leaves no
        // connection sending the value this one replaced
        // (`crates/accounting/spec/Connectors.lean` §8-2).
        self.connectors.invalidate(reference);
        let captured = SecretCaptured {
            reference: reference.clone(),
            origin: arrival.spelling().to_owned(),
        };
        self.record(EventKind::SecretCaptured, Payload::of(&captured)?)
    }

    /// The redemption closure the adapters take: one resolve per call,
    /// nothing cached, the lock held only while the vault is read.
    pub(in crate::worker) fn resolver(&self) -> gateway::SecretResolver {
        resolving(Arc::clone(&self.credentials.vault))
    }

    /// The vault this city resolves credentials through.
    ///
    /// Shared rather than lent, because one route resolves a credential
    /// off this thread: the transcription door runs on a socket task and
    /// must reach the same vault, so that "a credential is resolved at
    /// the last moment and exposed only while a header is written" stays
    /// one path rather than two.
    pub fn vault_handle(&self) -> Arc<std::sync::Mutex<gateway::Custodian>> {
        Arc::clone(&self.credentials.vault)
    }
}
