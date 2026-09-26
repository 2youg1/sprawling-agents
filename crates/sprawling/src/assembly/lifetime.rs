// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A worker opened over a history, and the city closed in the record.
//!
//! The two ends of one lifetime: `RunWorker::new`, `over` and
//! `holding` are LOADING - the worker folds, or is handed, what the
//! ledger says before it acts on anything - and `close_city` is UNLOADING, the one line that says a
//! stop was chosen rather than suffered. They sit together because a
//! reader asking "what does a restart find" and "what does a close
//! leave" is asking one question from two ends.

use super::{
    Collaborating, Credentials, Doorstep, Flight, GatewayModels, Planning, RoomQueues, RunWorker,
    Standing, SystemClock, city_segment,
};
use crate::doctor::{PATIENCE, Platform, ThisMachine};
use std::path::Path;

use kernel::{AxError, EventKind, Locator};
use memory::{Cas, JsonlLedger, OpenReport};

/// What opening the ledger repaired before this worker read a line of
/// it (sprawling-SPEC.md 8-90). The ledger records the cut as
/// `log_truncated`, but no page draws that line, so the worker keeps
/// this value to tell the person at the two doors a city opens through.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum LedgerOpening {
    /// Every byte on disk was a whole, chained line.
    Intact,
    /// A crash left the last line half-written; opening cut these bytes.
    TailDropped { bytes: u64 },
}

impl From<OpenReport> for LedgerOpening {
    fn from(report: OpenReport) -> Self {
        match report.recovered {
            None => LedgerOpening::Intact,
            Some(cut) => LedgerOpening::TailDropped {
                bytes: cut.dropped_bytes,
            },
        }
    }
}

impl LedgerOpening {
    /// The one sentence a person reads about the cut: what, why, and
    /// what to do. `None` when nothing was cut.
    pub(crate) fn notice(self) -> Option<String> {
        match self {
            LedgerOpening::Intact => None,
            LedgerOpening::TailDropped { bytes } => Some(format!(
                "the ledger's last line was half-written when the city last stopped, so \
                 opening cut {bytes} byte(s) from its tail and recorded log_truncated; every \
                 earlier line verified. Check whether the last action before that stop needs \
                 doing again"
            )),
        }
    }
}

/// Why the city is closing, carried from whoever decided it to the
/// line that records it.
///
/// Exhaustive rather than a flag, because the handoff says a different
/// thing for each: a close the person chose and a close serving forced
/// are different facts for the next session, and a record that spelled
/// both as the first would claim a choice nobody made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Closing {
    /// The person stopped the city from the keyboard.
    Chosen,
    /// Serving failed and took the city down; `cause` is what failed.
    Broken { cause: String },
}

impl Closing {
    /// The one reading of how serving ended: a serve that returned cleanly
    /// was ended by the person's Ctrl-C, and a serve that failed names its
    /// failure, so a failed serve is never recorded as the person's choice.
    pub(crate) fn of(served: &Result<(), AxError>) -> Self {
        match served {
            Ok(()) => Self::Chosen,
            Err(failure) => Self::Broken {
                cause: failure.to_string(),
            },
        }
    }
}

impl RunWorker {
    /// # Errors
    /// Propagates whatever opening the ledger or the store reports, and
    /// whatever the ledger says about its own chain: a worker that
    /// cannot read the city's history cannot know what is attached.
    pub fn new(
        city_root: &Path,
        vault: gateway::Custodian,
        log: runtime::diagnostics::Diagnostics,
    ) -> Result<Self, AxError> {
        let opened = JsonlLedger::open(
            &kernel::layout::CityLayout::new(city_root).ledger(),
            accounting::Clock::now(&SystemClock)?,
        )
        .map_err(memory::MemoryError::into_ax)?;
        RunWorker::over(city_root, vault, log, opened)
    }

    /// Builds a worker around a ledger somebody else opened, together
    /// with what that open repaired (LOADING; UNLOADING: `close_city`).
    ///
    /// Where the history comes from is not this worker's decision to
    /// make, and taking it as a parameter is the same correction
    /// ARCHITECTURE.md section 3 already asks for on the model adapter:
    /// a component that builds its own dependency cannot be driven
    /// against a second one. The ledger is a concrete `JsonlLedger` on
    /// both paths - the `Vfs` underneath it is what differs - so nothing
    /// here becomes a seam and no `pub trait` moves.
    ///
    /// # Errors
    /// Propagates whatever opening the store reports, and whatever the
    /// ledger says about its own chain: a worker that cannot read the
    /// city's history cannot know what is attached.
    pub(crate) fn over(
        city_root: &Path,
        vault: gateway::Custodian,
        log: runtime::diagnostics::Diagnostics,
        (ledger, report): (JsonlLedger, OpenReport),
    ) -> Result<Self, AxError> {
        let standing = Standing::fold(&kernel::layout::CityLayout::new(city_root).ledger())?;
        RunWorker::holding(city_root, vault, log, (ledger, report, standing))
    }

    ///
    /// `holding` takes a ledger already opened, its writer lock held, and
    /// what opening it repaired, and the standing folded from it under
    /// that lock.
    pub(crate) fn holding(
        city_root: &Path,
        vault: gateway::Custodian,
        log: runtime::diagnostics::Diagnostics,
        (ledger, report, standing): (JsonlLedger, OpenReport, Standing),
    ) -> Result<Self, AxError> {
        let now = accounting::Clock::now(&SystemClock)?;
        // Holding the one writer is what makes every worktree lock a
        // lock nobody alive holds (memory-SPEC 8-9).
        memory::Worktrees::lift_abandoned_leases(city_root, &ledger)
            .map_err(memory::MemoryError::into_ax)?;
        let Standing {
            book,
            governance,
            collaboration,
            entrance,
            expiries,
            origins,
        } = standing;
        let cas = Cas::open(&kernel::layout::CityLayout::new(city_root).cas())
            .map_err(memory::MemoryError::into_ax)?;
        // The one place a `Delegator` is minted in this process, which
        // is what makes "a sub-agent cannot set the city working" a
        // fact about the code rather than a rule somebody follows.
        let delegator = kernel::Delegator::root();
        let pursuits = collaboration.pursuits(&delegator);
        let mut worker = RunWorker {
            city_root: city_root.to_path_buf(),
            city: std::sync::OnceLock::new(),
            ledger,
            opening: LedgerOpening::from(report),
            cas,
            credentials: Credentials::opened(book, expiries, vault),
            serving: None,
            governance,
            collaborating: Collaborating {
                rooms: RoomQueues::folded(collaboration.inboxes),
                joins: collaboration.joins,
                workshops: std::collections::BTreeMap::new(),
                requests: collaboration.requests,
                goals: collaboration.goals,
            },
            planning: Planning {
                pursuits,
                delegator,
                holders: collaboration.plan_holders,
                write_plan: city::edit_against,
            },
            last_tick: now,
            log,
            doorstep: Doorstep::opened(entrance),
            origins,
            flight: Flight::open(),
            index: memory::LedgerIndex::empty(),
            warm: super::keeping_warm::Kept::default(),
            models: Box::new(GatewayModels),
            connectors: Box::new(super::mcp::Residents::default()),
            machine: Box::new(ThisMachine::new(Platform::current(), PATIENCE)),
            clock: std::sync::Arc::new(SystemClock),
        };
        worker.sweep_abandoned_trees();
        Ok(worker)
    }

    /// Takes back the trees a crash left behind (sprawling-SPEC 8-93).
    ///
    /// Nothing is held yet: the ledger this worker holds is locked to
    /// this process, and no run has been dispatched. A tree that will
    /// not go is told and left for the next open, because one stuck
    /// directory is no reason to keep the person out of their city.
    fn sweep_abandoned_trees(&mut self) {
        let told = match memory::Worktrees::sweep_abandoned(&self.city_root, &[]) {
            Ok(swept) if swept.is_empty() => return,
            Ok(swept) => format!(
                "took back {} worktree(s) a crash left behind: {}",
                swept.len(),
                swept
                    .iter()
                    .map(memory::WorktreeName::as_str)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Err(err) => format!(
                "could not take back the worktrees a crash left behind; the next open tries again: {err}"
            ),
        };
        self.note(runtime::diagnostics::Level::Effect, "bin::assembly", &told);
    }

    /// What opening this worker's ledger repaired.
    pub(crate) fn opening(&self) -> LedgerOpening {
        self.opening
    }

    /// Closes the city in the record, so a stop somebody chose, a stop
    /// serving forced, and a crash are three different records rather
    /// than one line and one silence.
    ///
    /// The five sections are the city's own: what the next session must
    /// read is the city's norms, and where it left off is the position
    /// the ledger stands at. Written through `runtime::handoff`, which
    /// is the one construction point for the shape - a hand-built
    /// payload here would be a second one.
    ///
    /// # Errors
    /// Propagates the handoff's refusal of an empty must-read list, and
    /// the ledger's refusal to take the line.
    pub(crate) fn close_city(&mut self, why: &Closing) -> Result<(), AxError> {
        // The city's own norm, not a building's: `city::norms` answers
        // for a run at an address, and this line belongs to the city.
        // Through the same reader the prefix uses. What this city's
        // norms are has one answer, and hashing zero bytes here made the
        // must-read locator point at nothing while the handoff went on
        // saying the next session must read them.
        let mut must_read = Vec::new();
        let bytes = city_segment(&self.city_root)?.bytes;
        let hash = self.cas.put(&bytes).map_err(memory::MemoryError::into_ax)?;
        must_read.push(Locator::cas(hash));
        let standing = self.ledger.position();
        let (overview, context, next_step) = match why {
            Closing::Chosen => (
                "the city was closed by the person running it".to_owned(),
                "an orderly close, not a crash: nothing was interrupted mid-command".to_owned(),
                "`sprawling serve` on this directory continues from here".to_owned(),
            ),
            Closing::Broken { cause } => (
                format!("the city stopped because serving failed: {cause}"),
                "not a choice: serving failed, and the command in hand finished first".to_owned(),
                "fix what the failure names, then `sprawling serve` on this directory".to_owned(),
            ),
        };
        let handoff = runtime::handoff::Handoff::new(
            must_read,
            overview,
            format!("the ledger stands at {}", standing.value()),
            context,
            next_step,
        )?;
        self.note(
            runtime::diagnostics::Level::Effect,
            "bin::assembly",
            "the city is closing; its handoff is on the ledger",
        );
        self.record(EventKind::HandoffWritten, handoff.payload()?)
    }

    /// The same worker, reading every time through `clock` instead of
    /// the wall clock (accounting-SPEC.md 8-3).
    ///
    /// The door citysim and the tests drive a worker through: when
    /// things happen is theirs to script, while what the worker writes
    /// at those times stays its own. The ledger was opened before this
    /// door, at the wall clock's time.
    #[must_use]
    pub fn with_clock(
        self,
        clock: std::sync::Arc<dyn accounting::Clock + Send + Sync>,
    ) -> RunWorker {
        RunWorker { clock, ..self }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::Closing;
    use crate::assembly::{RunWorker, init_city};
    use kernel::{AxCode, AxError};
    use std::io::Write;

    /// A crash mid-write leaves half a line; the next open cuts it and
    /// the person who runs `resume` has to be told what was cut.
    #[test]
    fn a_torn_tail_is_told_in_the_startup_scan() {
        let dir = tempfile::tempdir().unwrap();
        init_city(dir.path()).unwrap();
        let segment =
            memory::ledger_segments_at(&kernel::layout::CityLayout::new(dir.path()).ledger())
                .unwrap()
                .pop()
                .unwrap();
        let torn = b"{\"v\":1,\"seq\":99,\"half";
        std::fs::OpenOptions::new()
            .append(true)
            .open(&segment)
            .unwrap()
            .write_all(torn)
            .unwrap();

        let mut worker = RunWorker::new(
            dir.path(),
            gateway::Custodian::in_memory(),
            runtime::diagnostics::Diagnostics::off(),
        )
        .unwrap();
        let summary = worker.startup_scan().unwrap().summary();

        assert!(
            summary.contains(&format!("{} byte(s)", torn.len())),
            "the scan must say how much of the tail was cut: {summary}"
        );
    }

    #[test]
    fn a_failed_serve_closes_broken_and_a_clean_one_closes_chosen() {
        let failure = AxError::failure(AxCode::StorageFatal, "serve the city", "port taken")
            .with_recovery("choose another port");
        assert_eq!(
            [Closing::of(&Err(failure.clone())), Closing::of(&Ok(()))],
            [
                Closing::Broken {
                    cause: failure.to_string()
                },
                Closing::Chosen
            ]
        );
    }

    /// A run whose process died keeps its tree until the city next
    /// opens; that open is the one point no other process can hold the
    /// city, and the sweep has to happen on it.
    #[test]
    fn a_crash_left_worktree_is_gone_once_the_city_opens() {
        let dir = tempfile::tempdir().unwrap();
        init_city(dir.path()).unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let signature = git2::Signature::now("city", "city@example.invalid").unwrap();
        let tree = repo
            .find_tree(repo.index().unwrap().write_tree().unwrap())
            .unwrap();
        repo.commit(Some("HEAD"), &signature, &signature, "base", &tree, &[])
            .unwrap();
        let trees = memory::Worktrees::open(dir.path()).unwrap();
        let name = memory::WorktreeName::parse("run-1").unwrap();
        drop(trees.claim(&name, &[]).unwrap());

        drop(
            RunWorker::new(
                dir.path(),
                gateway::Custodian::in_memory(),
                runtime::diagnostics::Diagnostics::off(),
            )
            .unwrap(),
        );

        assert_eq!(trees.live().unwrap(), Vec::new());
    }
}
