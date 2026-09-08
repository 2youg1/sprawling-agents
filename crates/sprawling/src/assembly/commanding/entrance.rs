// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The key every command carries, read before anything is done about it.
//!
//! The wire makes all 23 state-changing commands carry an `IdemKey` so
//! that a client which lost its socket may send the frame again without
//! doing the work twice. `kernel::gate::dedup` judges membership and
//! says the seen set is the caller's state; this is that caller
//! (sprawling-SPEC.md 8-41).
//!
//! Two things are held here, and they answer two different questions.
//! The set answers "has this city already done this"; the map answers
//! "and what did it say", so a repeat is told what the first ask was
//! told rather than whatever the first ask's own effect now makes true.

use std::collections::{BTreeMap, BTreeSet};

use kernel::{AxError, IdemKey, Payload};

/// The payload field a command's key travels in, so that a city which
/// restarts can find in its own history what it has already carried out.
pub(in crate::assembly) const IDEM_FIELD: &str = "idem";

#[derive(Default)]
pub(in crate::assembly) struct Entrance {
    /// Every key this city has answered, from its history and from this
    /// process. `BTreeSet` because it is on a decision path.
    seen: BTreeSet<IdemKey>,
    /// The refusals among them. A key that is `seen` and absent here
    /// was carried out, and the answer to its repeat is silence.
    refused: BTreeMap<IdemKey, AxError>,
    /// The key of the command being carried out right now, which is
    /// what stamps the records that command writes.
    carrying: Option<IdemKey>,
}

impl Entrance {
    /// What this city already answered to this key, if it answered at
    /// all. `Some(Ok)` means it was carried out; `Some(Err)` means it
    /// was refused, with the words it was refused in.
    pub(in crate::assembly) fn answered(&self, key: &IdemKey) -> Option<Result<(), AxError>> {
        match kernel::dedup(&self.seen, key) {
            kernel::DedupVerdict::Fresh => None,
            kernel::DedupVerdict::Duplicate => {
                Some(self.refused.get(key).cloned().map_or(Ok(()), Err))
            }
        }
    }

    /// Takes the key of the command about to be carried out.
    pub(in crate::assembly) fn begin(&mut self, key: IdemKey) {
        self.carrying = Some(key);
    }

    /// Remembers how that command ended, and lets the key go.
    ///
    /// A refusal is remembered as well as a success, because a client
    /// that retries a refused frame is owed the refusal it earned and
    /// not a second one manufactured by the first attempt.
    pub(in crate::assembly) fn settle(&mut self, outcome: &Result<(), AxError>) {
        let Some(key) = self.carrying.take() else {
            return;
        };
        self.seen.insert(key);
        if let Err(err) = outcome {
            self.refused.insert(key, err.clone());
        }
    }

    /// Puts the key of the command in flight on the record it is
    /// writing, so a restart reads back what this city has done.
    ///
    /// A record written outside a command - a schedule firing, a run
    /// settling on its own - carries no key, because no client is
    /// holding one for it.
    ///
    /// # Errors
    /// Propagates the payload's own refusal of a value it cannot carry.
    pub(in crate::assembly) fn stamp(&self, data: Payload) -> Result<Payload, AxError> {
        let Some(key) = self.carrying else {
            return Ok(data);
        };
        let mut map = data.as_map().clone();
        map.insert(
            IDEM_FIELD.to_owned(),
            serde_json::Value::String(key.to_string()),
        );
        Payload::new(map)
    }

    /// Folds one line of the history back in.
    ///
    /// Called from the one verified pass a worker already makes over the
    /// ledger when it opens, so recognising a repeat across a restart
    /// costs no second read of the history.
    pub(in crate::assembly) fn absorb(&mut self, data: &Payload) {
        let Some(value) = data.as_map().get(IDEM_FIELD) else {
            return;
        };
        // A line whose key this build cannot read is a line whose key
        // this build must not act on: it is skipped rather than guessed
        // at, which costs one repeated command and never a wrong one.
        if let Ok(key) = serde_json::from_value::<IdemKey>(value.clone()) {
            self.seen.insert(key);
        }
    }
}

/// The diagnostic line a repeat leaves behind.
///
/// Written at the effect floor rather than at the refusal floor: nothing
/// was refused, and a person reading the log needs to see that the city
/// recognised the repeat rather than that something went wrong.
pub(in crate::assembly) fn repeated(name: &str) -> String {
    format!("{name} arrived again under a key this city has answered; the first answer stands")
}
