// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Whose identity this city can call which model under: the part of the
//! worker's state that `endpoint_attached`, `model_selected`,
//! `endpoint_lost` and `secret_captured` change (`crates/sprawling/Spec.lean` §8-110).

use std::sync::{Arc, Mutex};

use kernel::{AxError, EventKind, Payload};

/// The credentials the worker holds, one value because one family of
/// records changes them and one question reads them.
pub(in crate::worker) struct Credentials {
    /// Every endpoint the person attached and every model they chose,
    /// folded from the ledger. The worker keeps its own copy because a
    /// dispatch needs it synchronously, before the record it just wrote
    /// has reached any observer.
    pub(in crate::worker) book: gateway::EndpointBook,
    /// The vault. Shared because a redemption closure outlives the call
    /// that builds it; the lock is held for one resolve at a time.
    pub(in crate::worker) vault: Arc<Mutex<gateway::Custodian>>,
}

impl Credentials {
    /// The credentials a worker opens with: the book its history folded
    /// to, and the vault it was handed.
    pub(in crate::worker) fn opened(
        book: gateway::EndpointBook,
        vault: gateway::Custodian,
    ) -> Credentials {
        Credentials {
            book,
            vault: Arc::new(Mutex::new(vault)),
        }
    }

    /// Folds one line the worker wrote, whoever it was written for.
    ///
    /// # Errors
    /// A credential record this build cannot read: the book would then
    /// state something the history does not.
    pub(in crate::worker) fn absorb(
        &mut self,
        kind: EventKind,
        run: kernel::RunId,
        addr: Option<&kernel::Address>,
        data: &Payload,
    ) -> Result<(), AxError> {
        self.book.absorb(kind, run, addr, data)
    }
}
