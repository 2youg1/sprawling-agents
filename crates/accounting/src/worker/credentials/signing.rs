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

use kernel::config::SearchConfiguration;
use kernel::event::record::SecretCaptured;
use kernel::{AxCode, AxError, EventKind, Payload, SecretRef};

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

    /// Deletes one reference's key from the vault, once nothing the city
    /// reads names it (`crates/accounting/spec/Worker.lean` §8-37).
    ///
    /// Nothing is recorded: no fold reads that a reference once held a
    /// value, and the book and the configuration, which say who uses
    /// which reference, stopped naming it before this.
    ///
    /// # Errors
    /// The reference grammar's refusal; `E_CONFIG_INVALID` naming the
    /// first place that still names the reference; a configuration that
    /// cannot be read; the custodian's refusal of a key the environment
    /// supplies; a poisoned vault lock.
    pub(in crate::worker) fn forget_secret(&mut self, reference: &str) -> Result<(), AxError> {
        let reference = kernel::SecretRef::parse(reference)?;
        if let Some(holder) = self.named_by(&reference)? {
            return Err(AxError::failure(
                AxCode::ConfigInvalid,
                "forget credential",
                format!("{reference} is still named by {holder}"),
            )
            .with_recovery(format!(
                "remove {reference} from {holder} first, then forget the key"
            )));
        }
        self.credentials
            .vault
            .lock()
            .map_err(|_| poisoned_vault())?
            .forget(&reference)?;
        // Same reason as `put_secret`: a standing connection redeemed the
        // key when it opened and still carries it.
        self.connectors.invalidate(&reference);
        Ok(())
    }

    /// The first place the city reads that names `reference`: an attached
    /// endpoint, the city's own `[search]`, then what each building's
    /// configuration puts in force. A room's own layer is not read; a
    /// room that names a forgotten key is told so by its next run.
    fn named_by(&self, reference: &kernel::SecretRef) -> Result<Option<String>, AxError> {
        if let Some(endpoint) = self
            .credentials
            .book
            .endpoints()
            .find(|endpoint| endpoint.references().contains(&reference))
        {
            return Ok(Some(format!("provider `{}`", endpoint.name)));
        }
        if let Some(supplier) = city::city_search(&self.city_root)?
            .as_ref()
            .and_then(|search| supplier_naming(search, reference))
        {
            return Ok(Some(format!("the city's [search] supplier `{supplier}`")));
        }
        for building in city::buildings(&self.city_root)? {
            let frozen = city::load_config(&self.city_root, &building)?;
            let at = building.as_str();
            if let Some(server) = frozen
                .mcp
                .iter()
                .find(|server| super::super::mcp::names(server, reference))
            {
                return Ok(Some(format!(
                    "[[mcp]] `{}` at `{at}`",
                    server.label.as_str()
                )));
            }
            if let Some(supplier) = supplier_naming(&frozen.search, reference) {
                return Ok(Some(format!("[search] supplier `{supplier}` at `{at}`")));
            }
        }
        Ok(None)
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

/// The id of the `[search]` supplier one of whose accounts calls with
/// `reference`, if any.
fn supplier_naming<'a>(search: &'a SearchConfiguration, reference: &SecretRef) -> Option<&'a str> {
    match search {
        SearchConfiguration::Custom { suppliers, .. } => suppliers
            .iter()
            .find(|supplier| {
                supplier
                    .accounts
                    .iter()
                    .any(|account| account.reference.as_ref() == Some(reference))
            })
            .map(|supplier| supplier.id.as_str()),
        SearchConfiguration::Default | SearchConfiguration::Off => None,
    }
}
