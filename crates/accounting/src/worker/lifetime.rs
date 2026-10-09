// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A worker opened over a history: the LOADING end of one lifetime.
//!
//! `RunWorker::new`, `over` and `holding` fold, or are handed, what the
//! ledger says before the worker acts on anything, and the two sweeps
//! beside them take back what a crash left. The other end is
//! [`closing`], which records why the city stopped. They sit together
//! because a reader asking "what does a restart find" and "what does a
//! close leave" is asking one question from two ends.

mod closing;

pub use closing::{ClosedBy, Closing};

use super::{
    Collaborating, Credentials, Doorstep, Flight, GatewayModels, Hands, Planning, RoomQueues,
    RunWorker, Standing,
};
use std::path::Path;

use kernel::AxError;
use storage::{Cas, JsonlLedger, OpenReport};

/// What opening the ledger repaired before this worker read a line of
/// it (`crates/sprawling/Spec.lean` §8-102). The ledger records the cut as
/// `log_truncated`, but no page draws that line, so the worker keeps
/// this value to tell the person at the two doors a city opens through.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LedgerOpening {
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
    pub fn notice(self) -> Option<String> {
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

impl RunWorker {
    /// Opens the city's ledger at the time `hands.clock` reads, and
    /// builds the worker over it with `hands` (`crates/accounting/spec/Worker.lean` §8-11).
    ///
    /// # Errors
    /// Propagates whatever opening the ledger or the store reports, and
    /// whatever the ledger says about its own chain: a worker that
    /// cannot read the city's history cannot know what is attached.
    pub fn new(
        city_root: &Path,
        log: runtime::diagnostics::Diagnostics,
        hands: Hands,
    ) -> Result<Self, AxError> {
        let opened = JsonlLedger::open(
            &kernel::layout::CityLayout::new(city_root).ledger(),
            crate::Clock::now(&*hands.clock)?,
        )
        .map_err(storage::StorageError::into_ax)?;
        RunWorker::over(city_root, log, hands, opened)
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
    pub fn over(
        city_root: &Path,
        log: runtime::diagnostics::Diagnostics,
        hands: Hands,
        (ledger, report): (JsonlLedger, OpenReport),
    ) -> Result<Self, AxError> {
        let standing = Standing::fold(&kernel::layout::CityLayout::new(city_root).ledger())?;
        RunWorker::holding(city_root, log, hands, (ledger, report, standing))
    }

    /// `holding` takes a ledger already opened, its writer lock held, and
    /// what opening it repaired, and the standing folded from it under
    /// that lock. The schedule starts from the time `hands.clock` reads.
    pub fn holding(
        city_root: &Path,
        mut log: runtime::diagnostics::Diagnostics,
        hands: Hands,
        (ledger, report, standing): (JsonlLedger, OpenReport, Standing),
    ) -> Result<Self, AxError> {
        let Hands {
            vault,
            clock,
            monotonic,
            machine,
            read_memory,
            read_volume,
            reveal,
            browsers,
            desktop_program,
            recipe_for,
            exec_host,
            seat_lane,
            shares,
            affinity,
        } = hands;
        let now = crate::Clock::now(&*clock)?;
        // Holding the one writer is what makes every worktree lock a
        // lock nobody alive holds (`crates/storage/Spec.lean` §8-9).
        storage::Worktrees::lift_abandoned_leases(city_root, &ledger)
            .map_err(storage::StorageError::into_ax)?;
        // A rule the ignore table gained since a building was raised
        // reaches it now: no working record of this city enters git,
        // however old the building (city D5).
        city::keep_records_out_of_git(city_root)?;
        let Standing {
            book,
            governance,
            collaboration,
            entrance,
            origins,
            cut,
        } = standing;
        if let Err(fault) = cut {
            log.write(
                runtime::diagnostics::Level::Refuse,
                runtime::diagnostics::Site {
                    run: kernel::RunId::CITY,
                    seq: ledger.position(),
                    module: "accounting::worker",
                },
                &format!(
                    "the standing snapshot was not cut: {fault}; the city goes on, and the next start folds from the older snapshot or from genesis"
                ),
            );
        }
        let cas = Cas::open(&kernel::layout::CityLayout::new(city_root).cas())
            .map_err(storage::StorageError::into_ax)?;
        let lane_store = std::sync::Arc::new(std::sync::Mutex::new(
            Cas::open(&kernel::layout::CityLayout::new(city_root).cas())
                .map_err(storage::StorageError::into_ax)?,
        ));
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
            lane_store,
            credentials: Credentials::opened(book, vault),
            serving: None,
            governance,
            collaborating: Collaborating {
                rooms: RoomQueues::folded(collaboration.inboxes),
                joins: collaboration.joins,
                workshops: std::collections::BTreeMap::new(),
                handing: std::collections::BTreeMap::new(),
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
            log: super::recording::Notes::over(log),
            doorstep: Doorstep::opened(entrance),
            origins,
            flight: Flight::open(
                read_memory,
                monotonic,
                seat_lane,
                runtime::Backlog::new()
                    .with_shares(shares)
                    .with_affinity(affinity),
            ),
            index: storage::LedgerIndex::empty(),
            warm: super::keeping_warm::Kept::default(),
            models: Box::new(GatewayModels { monotonic }),
            connectors: std::sync::Arc::new(super::mcp::Residents::default()),
            machine,
            clock,
            monotonic,
            read_volume,
            reveal,
            browsers,
            desktop_program,
            harnesses: super::driving::harness::on_this_machine(),
            recipe_for,
            exec_host,
        };
        worker.sweep_abandoned_trees();
        worker.open_checkpoints();
        Ok(worker)
    }

    /// Takes back the trees a crash left behind (`crates/sprawling/Spec.lean` §8-108).
    ///
    /// Nothing is held yet: the ledger this worker holds is locked to
    /// this process, and no run has been dispatched. A tree that will
    /// not go is told and left for the next open, because one stuck
    /// directory is no reason to keep the person out of their city.
    fn sweep_abandoned_trees(&mut self) {
        let told = match storage::Worktrees::sweep_abandoned(&self.city_root, &[]) {
            Ok(swept) if swept.is_empty() => return,
            Ok(swept) => format!(
                "took back {} worktree(s) a crash left behind: {}",
                swept.len(),
                swept
                    .iter()
                    .map(storage::WorktreeName::as_str)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Err(err) => format!(
                "could not take back the worktrees a crash left behind; the next open tries again: {err}"
            ),
        };
        self.note(
            runtime::diagnostics::Level::Effect,
            "accounting::worker",
            &told,
        );
    }

    /// Makes the checkpoint repository before a lane asks for a writer's index, and takes back
    /// the indexes a crash left: no run is live yet (`crates/sprawling/spec/Accounting/Views.lean` §8-46-13).
    fn open_checkpoints(&mut self) {
        let opened = storage::Checkpoint::open(&self.city_root)
            .and_then(|_| storage::Checkpoint::sweep_writers(&self.city_root));
        let told = match opened {
            Ok(0) => return,
            Ok(swept) => format!("took back {swept} checkpoint index(es) a crash left behind"),
            Err(err) => format!(
                "could not open the checkpoint repository or take back the indexes a crash left; the next open tries again: {err}"
            ),
        };
        let level = runtime::diagnostics::Level::Effect;
        self.note(level, "accounting::worker", &told);
    }

    /// What opening this worker's ledger repaired.
    pub fn opening(&self) -> LedgerOpening {
        self.opening
    }

    /// The same worker, reading every time through `clock` instead of
    /// the wall clock (`crates/accounting/spec/Clock.lean` §8-3).
    ///
    /// The door citysim and the tests drive a worker through: when
    /// things happen is theirs to script, while what the worker writes
    /// at those times stays its own. The ledger was opened before this
    /// door, at the time the clock in the worker's `Hands` read.
    #[must_use]
    pub fn with_clock(self, clock: std::sync::Arc<dyn crate::Clock + Send + Sync>) -> RunWorker {
        RunWorker { clock, ..self }
    }

    /// The same worker, taking the browser tools a building's rules ask
    /// for from `browsers` instead of starting a browser on this host
    /// (`crates/sprawling/Spec.lean` §8-45-2).
    #[must_use]
    pub fn with_browsers(self, browsers: super::Browsers) -> RunWorker {
        RunWorker { browsers, ..self }
    }

    /// The same worker, starting the desktop server a building's rules
    /// ask for from `program` instead of from this executable
    /// (`crates/sprawling/Spec.lean` §8-4d).
    #[must_use]
    pub fn with_desktop_program(self, program: super::DesktopProgram) -> RunWorker {
        RunWorker {
            desktop_program: program,
            ..self
        }
    }

    /// The same worker, starting the harness a room's resident names
    /// through `start` instead of on this host (`crates/sprawling/Spec.lean` §8-124).
    /// Only a test plays a harness; production starts the vendor's own.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn with_harnesses(self, start: super::driving::harness::StartHarness) -> RunWorker {
        RunWorker {
            harnesses: start,
            ..self
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use crate::worker::RunWorker;
    use std::io::Write;

    /// A crash mid-write leaves half a line; the next open cuts it and
    /// the person who runs `resume` has to be told what was cut.
    #[test]
    fn a_torn_tail_is_told_in_the_startup_scan() {
        let dir = tempfile::tempdir().unwrap();
        crate::worker::fixture::init_city(dir.path()).unwrap();
        let segment =
            storage::ledger_segments_at(&kernel::layout::CityLayout::new(dir.path()).ledger())
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
            runtime::diagnostics::Diagnostics::off(),
            crate::worker::fixture::hands(),
        )
        .unwrap();
        let summary = worker.startup_scan().unwrap().summary();

        assert!(
            summary.contains(&format!("{} byte(s)", torn.len())),
            "the scan must say how much of the tail was cut: {summary}"
        );
    }

    /// A run whose process died keeps its tree until the city next
    /// opens; that open is the one point no other process can hold the
    /// city, and the sweep has to happen on it.
    #[test]
    fn a_crash_left_worktree_is_gone_once_the_city_opens() {
        let dir = tempfile::tempdir().unwrap();
        crate::worker::fixture::init_city(dir.path()).unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let signature = git2::Signature::now("city", "city@example.invalid").unwrap();
        let tree = repo
            .find_tree(repo.index().unwrap().write_tree().unwrap())
            .unwrap();
        repo.commit(Some("HEAD"), &signature, &signature, "base", &tree, &[])
            .unwrap();
        let trees = storage::Worktrees::open(dir.path()).unwrap();
        let name = storage::WorktreeName::parse("run-1").unwrap();
        drop(trees.claim(&name, &[]).unwrap());

        drop(
            RunWorker::new(
                dir.path(),
                runtime::diagnostics::Diagnostics::off(),
                crate::worker::fixture::hands(),
            )
            .unwrap(),
        );

        assert_eq!(trees.live().unwrap(), Vec::new());
    }
}
