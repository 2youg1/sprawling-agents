// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The lines this worker appends, and the fold each one is shown to.
//!
//! Three entrances and one rule: the append comes first, and the
//! worker's own books - the governance fold, the endpoint book - are
//! shown the line only once the history has it. A book that stated
//! what the process hoped to write would be a second authority.

use kernel::{Address, AxError, EventDraft, EventKind, Ledger, Payload, RunId};

use crate::effect;

use super::{RunWorker, now_ms};

impl RunWorker {
    /// Writes one diagnostic line, anchored to where the ledger stands.
    pub(super) fn note(&mut self, level: runtime::diagnostics::Level, module: &str, message: &str) {
        let site = runtime::diagnostics::Site {
            run: RunId::CITY,
            seq: self.ledger.position(),
            module,
        };
        self.log.write(level, site, message);
    }

    /// Appends one city record and folds it into the worker's own book.
    /// The append comes first: the book states what the history says,
    /// never what the process hoped to write.
    pub(super) fn record(&mut self, kind: EventKind, data: Payload) -> Result<(), AxError> {
        self.record_where(kind, None, data)
    }

    /// Writes the line that says which run this one continues, and
    /// marks the session's inheritance spent.
    ///
    /// Appended directly rather than through [`Self::record_at`], because
    /// a lineage belongs to the *new* run and not to the city: the
    /// record's own `run` field is what says which run continues which,
    /// and `record_at` speaks for the city.
    ///
    /// # Errors
    /// Propagates a ledger that refuses the append, and the payload
    /// failing to encode - by then the run has been frozen and the line
    /// is what a page reads the branch from, so the refusal is raised
    /// rather than swallowed.
    pub(super) fn note_lineage(
        &mut self,
        addr: &Address,
        run: RunId,
        origin: kernel::Origin,
    ) -> Result<(), AxError> {
        let t = now_ms()?;
        let draft = runtime::fork::fork_draft(origin, run, addr.clone(), t, "owner".to_owned())?;
        self.ledger.append(draft)?;
        self.origins.spent(addr);
        Ok(())
    }

    /// The same, for a record that belongs to one address. A pursuit is
    /// a building's, and a record with no address would be a fact about
    /// the city that no view could file under the building it changed.
    pub(super) fn record_at(
        &mut self,
        kind: EventKind,
        addr: Address,
        data: Payload,
    ) -> Result<(), AxError> {
        self.record_where(kind, Some(addr), data)
    }

    fn record_where(
        &mut self,
        kind: EventKind,
        addr: Option<Address>,
        data: Payload,
    ) -> Result<(), AxError> {
        // The key of the command in flight goes on the record it is
        // writing, and nowhere else: that is how a restarted city reads
        // out of its own history what it has already carried out.
        let data = self.entrance.stamp(data)?;
        let draft = EventDraft {
            run: RunId::CITY,
            t: now_ms()?,
            who: "owner".to_owned(),
            addr: addr.clone(),
            kind,
            data: data.clone(),
            ig: false,
        };
        self.ledger.append(draft)?;
        // Every fold that reads a record this worker writes is shown it
        // here, because the worker's own books are what its next decision
        // reads: a fold updated only on the rebuild path would answer a
        // dispatch about a session the process has already recorded.
        self.absorb(kind, RunId::CITY, addr.as_ref(), &data)
    }

    /// Appends one line attributed to a run rather than to the city.
    /// Separate from `record` because that one speaks for the city: an
    /// effect a resident caused must carry the resident's name, or the
    /// history cannot say who spoke.
    pub(super) fn record_for(&mut self, run: RunId, line: effect::Line) -> Result<(), AxError> {
        let effect::Line {
            who,
            addr,
            kind,
            data,
        } = line;
        let data = self.entrance.stamp(data)?;
        self.ledger.append(EventDraft {
            run,
            t: now_ms()?,
            who,
            addr: Some(addr.clone()),
            kind,
            data: data.clone(),
            ig: false,
        })?;
        self.absorb(kind, run, Some(&addr), &data)
    }

    /// Shows one line this worker wrote to every fold it holds, whoever
    /// the line was written for: a restart folds every line into every
    /// fold, so a live fold that skipped some writer's lines would
    /// disagree with the restart until the process restarted
    /// (sprawling-SPEC.md 8-90). Each fold's own `absorb` decides which
    /// kinds it reads.
    fn absorb(
        &mut self,
        kind: EventKind,
        run: RunId,
        addr: Option<&Address>,
        data: &Payload,
    ) -> Result<(), AxError> {
        self.origins.absorb(kind, addr, data)?;
        self.governance.absorb(kind, run, addr, data)?;
        self.planning.absorb(kind, addr, data);
        self.credentials.absorb(kind, data)
    }
}
