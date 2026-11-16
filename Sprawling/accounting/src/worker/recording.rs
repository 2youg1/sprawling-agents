// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The lines this worker appends, and the fold each one is shown to.
//!
//! Four entrances and one rule: the append comes first, and the
//! worker's own books - the governance fold, the endpoint book - are
//! shown the line only once the history has it. A book that stated
//! what the process hoped to write would be a second authority.

use kernel::{Address, AxError, EventDraft, EventKind, Ledger, Payload, RunId};

use crate::effect;
use crate::worker::booking::OpenClaims;

use super::RunWorker;

/// A ledger a dispatch's preparation writes through, with the key of the
/// command that dispatch answers, so a line written without the worker
/// is stamped by the rule [`RunWorker::record_for`] follows and a restart
/// still recognises the command from it (`crates/sprawling/Spec.lean` §8-113).
pub(in crate::worker) struct Stamping<'a, L> {
    pub(in crate::worker) ledger: &'a mut L,
    pub(in crate::worker) command: Option<kernel::IdemKey>,
    /// What time it is, for each line's stamp.
    pub(in crate::worker) clock: &'a (dyn crate::Clock + Send + Sync),
}

impl<L: Ledger> Stamping<'_, L> {
    /// Appends one line on behalf of `run`, stamped with the command's
    /// key when there is one.
    ///
    /// # Errors
    /// Propagates a payload that will not take the key, a clock this
    /// machine will not read, and the ledger's refusal of the line.
    pub(in crate::worker) fn record_for(
        &mut self,
        run: RunId,
        line: effect::Line,
    ) -> Result<(), AxError> {
        let effect::Line {
            who,
            addr,
            kind,
            data,
        } = line;
        self.ledger.append(EventDraft {
            run,
            t: self.clock.now()?,
            who,
            addr: Some(addr),
            kind,
            data: super::commanding::entrance::stamped(self.command, data)?,
            ig: false,
        })?;
        Ok(())
    }
}

/// The diagnostic log's write end: the worker holds one, and each lane
/// preparing a dispatch holds a clone (`crates/sprawling/Spec.lean` §8-113).
///
/// Shared rather than lent, because nothing reads a line back: where a
/// line lands decides nothing, so two threads writing to one log need
/// only take turns.
#[derive(Clone)]
pub(in crate::worker) struct Notes(
    std::sync::Arc<std::sync::Mutex<runtime::diagnostics::Diagnostics>>,
);

impl Notes {
    pub(in crate::worker) fn over(log: runtime::diagnostics::Diagnostics) -> Notes {
        Notes(std::sync::Arc::new(std::sync::Mutex::new(log)))
    }

    /// Writes one line for the city, anchored at `seq`.
    ///
    /// A lock a dead thread left behind is written through all the
    /// same: every write is one step under the lock, so the log inside
    /// it is whole, and losing the line would hide the fault it names.
    pub(in crate::worker) fn write(
        &self,
        level: runtime::diagnostics::Level,
        seq: kernel::Seq,
        module: &str,
        message: &str,
    ) {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .write(
                level,
                runtime::diagnostics::Site {
                    run: RunId::CITY,
                    seq,
                    module,
                },
                message,
            );
    }
}

impl RunWorker {
    /// Writes one diagnostic line, anchored to where the ledger stands.
    pub(super) fn note(&mut self, level: runtime::diagnostics::Level, module: &str, message: &str) {
        self.log
            .write(level, self.ledger.position(), module, message);
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
        let t = self.clock.now()?;
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
        let data = super::commanding::entrance::stamped(self.doorstep.entrance.carrying(), data)?;
        let draft = EventDraft {
            run: RunId::CITY,
            t: self.clock.now()?,
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
        let (kind, addr, data) = self.append_for(run, line)?;
        self.absorb(kind, run, Some(&addr), &data)
    }

    /// Appends one line of a run's plan step and closes the node it
    /// closes in `open` as soon as the history has the line, before any
    /// fold is shown it: a fold that refuses the line afterwards cannot
    /// take it off the ledger, so a hand-back owed for that node would
    /// make the history say a finished node was handed back
    /// (`crates/sprawling/Spec.lean` §8-42-8).
    pub(super) fn record_closing(
        &mut self,
        run: RunId,
        closing: effect::Closing,
        open: &mut OpenClaims,
    ) -> Result<(), AxError> {
        let (kind, addr, data) = self.append_for(run, closing.line)?;
        if let Some(node) = &closing.closes {
            open.close(node);
        }
        self.absorb(kind, run, Some(&addr), &data)
    }

    /// Appends one line attributed to a run, and hands back what the
    /// folds are shown: its kind, its room and the stamped payload.
    fn append_for(
        &mut self,
        run: RunId,
        line: effect::Line,
    ) -> Result<(EventKind, Address, Payload), AxError> {
        let effect::Line {
            who,
            addr,
            kind,
            data,
        } = line;
        let data = super::commanding::entrance::stamped(self.doorstep.entrance.carrying(), data)?;
        self.ledger.append(EventDraft {
            run,
            t: self.clock.now()?,
            who,
            addr: Some(addr.clone()),
            kind,
            data: data.clone(),
            ig: false,
        })?;
        Ok((kind, addr, data))
    }

    /// Shows every line the gate wrote for the lanes and the claims to
    /// the folds, in ledger order. Each line is already history and its
    /// lane already has its answer, so a fold that refuses one is
    /// reported: no lane is left to act on the refusal, and the next
    /// line must still be shown (`crates/sprawling/Spec.lean` §8-110).
    pub(super) fn show_relayed(&mut self, written: Vec<EventDraft>) {
        let runs: Vec<RunId> = written.iter().map(|line| line.run).collect();
        for line in written {
            if let Err(err) = self.absorb(line.kind, line.run, line.addr.as_ref(), &line.data) {
                self.note(
                    runtime::diagnostics::Level::Refuse,
                    "crate::worker::relay",
                    &format!("a fold refused a {:?} line a lane wrote: {err}", line.kind),
                );
            }
            if let Err(err) = self.deliver_sent(&line) {
                self.note(
                    runtime::diagnostics::Level::Refuse,
                    "collab::inbox",
                    &format!("a signal a lane sent reached no room: {err}"),
                );
            }
        }
        // What a call handed down starts now, while its run still drives
        // (collab D7); each run once, in the order its lines arrived.
        let mut seen = Vec::new();
        for run in runs {
            if seen.contains(&run) {
                continue;
            }
            seen.push(run);
            if let Err(err) = self.hand_over_at_call(run) {
                self.note(
                    runtime::diagnostics::Level::Refuse,
                    "collab::delegate",
                    &format!("work {run} handed down could not be read: {err}"),
                );
            }
        }
        // A knock the delivery queued starts its run now, while the
        // sender is still driving (collab D7).
        self.answer_knocks();
    }

    /// Shows one line this worker wrote to the origins, governance,
    /// planning, goal and credentials folds, whoever the line was written
    /// for: a restart folds every line into every fold, so a live fold
    /// that skipped some writer's lines would disagree with the restart
    /// until the process restarted (`crates/sprawling/Spec.lean` §8-110). Each fold's
    /// own `absorb` decides which kinds it reads. The other collaboration
    /// fields and the entrance are not shown the line here: the effect
    /// handler that wrote it and `entrance.stamp` keep them in step.
    ///
    /// # Errors
    /// The first fold's failure, returned only after every fold has seen
    /// the line: it is already on the ledger, so a fold that missed it
    /// would answer differently from a restart.
    fn absorb(
        &mut self,
        kind: EventKind,
        run: RunId,
        addr: Option<&Address>,
        data: &Payload,
    ) -> Result<(), AxError> {
        [
            self.origins.absorb(kind, run, addr, data),
            self.governance.absorb(kind, run, addr, data),
            self.planning.absorb(kind, addr, data),
            self.flight.gate.booked.absorb(kind, addr, data),
            super::collaborating::register_goal(&mut self.collaborating.goals, kind, data),
            self.credentials.absorb(kind, run, addr, data),
        ]
        .into_iter()
        .collect()
    }
}
