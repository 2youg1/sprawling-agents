// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What one run works at: where it stands, and the bench it is given.
//!
//! The values are declared here and the methods that build them live
//! below: `standing` settles the site, `desks` opens what the run
//! writes to, `tools` lays out the bench, `servers` connects what the
//! building's configuration names, and `engine` is the machine's own
//! half of `exec`. A child reads its parent's private fields, so the
//! split opened none of them.

use std::path::{Path, PathBuf};

use kernel::{Address, AxCode, AxError, RunId};
use runtime::bench::ToolBench;

use kernel::event::record::autonomy_word;

mod desks;
mod desktop;
mod engine;
mod servers;
mod standing;
mod tools;

pub(super) use standing::{Lending, Placing, lend_tree};

/// What laying out a run's bench reads from the city: handles that
/// clone, and values read when the dispatch was staged, so the bench is
/// laid out in the lane that drives the run (`crates/sprawling/Spec.lean` §8-113).
///
/// The folds the accounting thread rewrites - who has mail waiting, who
/// holds which goal, how far the city trusts its residents - arrive as
/// the values they had when the dispatch was staged, which is the moment
/// the worker read them before the bench moved into the lane.
pub(in crate::worker) struct Laying {
    pub(in crate::worker) city_root: PathBuf,
    /// The city's genesis line, which signs every checkpoint.
    pub(in crate::worker) city: kernel::B3Hash,
    vault: std::sync::Arc<std::sync::Mutex<gateway::Custodian>>,
    connectors: std::sync::Arc<dyn crate::Connectors + Send + Sync>,
    /// The browser tools a building's rules ask for (`RunWorker::browsers`).
    browsers: super::Browsers,
    /// Where the desktop server a building's rules ask for is started
    /// from (`RunWorker::desktop_program`).
    desktop_program: super::DesktopProgram,
    exec_host: super::hands::ExecHost,
    backlog: runtime::Backlog,
    /// The city's one checkpoint at a time (`driving::lane::DriveContext`).
    pub(in crate::worker) checkpoint_gate: std::sync::Arc<std::sync::Mutex<()>>,
    /// The store the lanes share (`RunWorker::lane_store`).
    pub(in crate::worker) store: std::sync::Arc<std::sync::Mutex<storage::Cas>>,
    pub(in crate::worker) notes: super::recording::Notes,
    /// How long starting the building's servers took (`Hands.monotonic`).
    monotonic: fn() -> std::time::Instant,
    /// Where the ledger stood when the dispatch was staged: what the
    /// keeper counts from, and where each line written here is anchored.
    pub(in crate::worker) staged_at: kernel::Seq,
    trust: kernel::Autonomy,
    /// How many signals each room with any waiting was holding.
    waiting: std::collections::BTreeMap<Address, u32>,
    /// The goals this run's resident already holds, which `status`
    /// answers from the list the conflict check reads.
    locks: Vec<String>,
    /// The endpoints and the choices made from them, which the
    /// transcription and OCR tools ask under this building's rules
    /// (`crates/sprawling/Spec.lean` §8-131, §8-142).
    book: gateway::EndpointBook,
    proposing: tools::proposal::Proposing,
}

impl Laying {
    /// Writes one diagnostic line, anchored where the dispatch was staged.
    fn note(&self, level: runtime::diagnostics::Level, module: &str, message: &str) {
        self.notes.write(level, self.staged_at, module, message);
    }

    /// Opens the checkpoint a run's writes are checkpointed in.
    ///
    /// The first open in a city creates its repository, and two lanes
    /// creating one race on its config lock, so the open takes the gate
    /// a checkpoint takes (`crates/sprawling/Spec.lean` §8-46-13).
    ///
    /// # Errors
    /// Propagates a gate a dead thread left, and a repository that will
    /// not open.
    fn open_checkpoint(&self, root: &std::path::Path) -> Result<storage::Checkpoint, AxError> {
        let turn = held(&self.checkpoint_gate, "take the checkpoint gate")?;
        let opened = storage::Checkpoint::open(root).map_err(storage::StorageError::into_ax);
        drop(turn);
        opened
    }
}

impl super::RunWorker {
    /// What laying out a bench for `who` reads from this worker, taken
    /// now.
    ///
    /// # Errors
    /// Propagates a city whose genesis line cannot be read.
    pub(in crate::worker) fn laying(&self, who: &str) -> Result<Laying, AxError> {
        Ok(Laying {
            city_root: self.city_root.clone(),
            city: self.city_hash()?,
            vault: self.vault_handle(),
            connectors: std::sync::Arc::clone(&self.connectors),
            browsers: self.browsers,
            desktop_program: self.desktop_program,
            exec_host: self.exec_host,
            backlog: self.flight.backlog.clone(),
            checkpoint_gate: std::sync::Arc::clone(&self.flight.checkpoint_gate),
            store: std::sync::Arc::clone(&self.lane_store),
            notes: self.log.clone(),
            monotonic: self.monotonic,
            staged_at: self.ledger.position(),
            trust: self.governance.autonomy.clone(),
            waiting: self.collaborating.rooms.waiting(),
            locks: self
                .collaborating
                .goals
                .iter()
                .filter(|entry| entry.owner == who)
                .map(|entry| entry.statement.clone())
                .collect(),
            book: self.credentials.book.clone(),
            proposing: self.proposing(),
        })
    }
}

/// Who checks a delegate's own done check. Not the delegate: the whole
/// point of `Claim::verified` is that a producer's verdict on its own
/// work is not verification.
pub(super) const CITY_VERIFIER: &str = "city";

/// Where one run stands before anything is built for it: the rules over
/// it, the model it may reach, who it runs as, and the tree it writes in.
///
/// One value rather than three, because the three readings of this phase
/// interlock: the lease is named after the run, the run's id is minted
/// after a credential renewal that may go to the network, and that
/// renewal is chosen by the building's own rules. Splitting them would
/// move a clock sample, and where the one sampling point lands is not
/// something a structural change may alter (ARCHITECTURE.md section 10).
pub(super) struct Site {
    pub(super) building: city::Building,
    pub(super) rules: city::BuildingRules,
    /// City, building and resident layers, resolved once and frozen for
    /// the whole run.
    pub(super) config: kernel::FrozenConfig,
    pub(super) model: gateway::ModelEntry,
    /// The endpoint the model is reached through (`crates/sprawling/Spec.lean` §8-85).
    pub(super) provider: String,
    pub(super) adapter: Option<super::keeping_warm::Door>,
    pub(super) identity: city::Identity,
    pub(super) who: String,
    pub(super) run_id: RunId,
    /// The run this one replaces, when a resident succeeded itself.
    /// Signed into every checkpoint this run raises, so a commit can be
    /// asked for its lineage.
    pub(super) predecessor: Option<RunId>,
    /// Some when the building asks for review: the tree this run writes
    /// in, which goes back whether the run finished or failed.
    pub(super) lease: Option<storage::WorktreeLease>,
    pub(super) write_root: PathBuf,
    branch: Option<String>,
    /// What survives a command's output in this run, resolved from the
    /// city's and the building's `FILTERS.toml` and frozen with the run
    /// (`crates/sprawling/Spec.lean` §8-43).
    pub(super) filters: runtime::FilterTable,
    /// The moment this run's driver read its clock last, which `status`
    /// and a command's clock line report (`crates/sprawling/Spec.lean` §8-125).
    pub(super) clock: runtime::ClockReading,
    /// Carried from the endpoint this run was given, frozen with
    /// everything else the run was set up with.
    pub(super) retries: kernel::Retries,
}

/// What the model may see, what routes what it calls, and who it may
/// hand work down to.
///
/// `ToolBench` routes one call; this is the whole bench a run works at:
/// the catalogue the model was told about, the router behind it, and
/// the delegate desk two of the tools write to. The catalogue is kept
/// rather than the tool definitions it renders, so there is one answer
/// to what this run admits rather than a list and a copy of it.
pub(super) struct Workbench {
    pub(super) catalog: std::sync::Arc<std::sync::Mutex<runtime::Catalog>>,
    /// The bench, until the drive takes it. `Option` is the vehicle of
    /// that move and not a second state - the same handling `Site`
    /// gives its adapter - because a drive owns everything it runs on
    /// and may leave this thread with it (`crates/sprawling/Spec.lean` §8-46-1).
    bench: Option<ToolBench>,
    pub(super) delegates: std::sync::Arc<std::sync::Mutex<collab::DelegateDesk>>,
    /// Whether this run asked to be replaced, read when it concludes.
    pub(super) succession: std::sync::Arc<std::sync::Mutex<runtime::SuccessionDesk>>,
    /// Where the run records the provider's count, which `status` reads.
    pub(super) context: runtime::ContextReading,
}

/// Who this run can reach: the residents beside it, and the sub-agents
/// under it.
///
/// Both are read by one answer - what `status` tells the model about the
/// city around it - so they arrive as one value rather than as two
/// parameters a caller could hand over half of.
pub(super) struct Reach<'a> {
    pub(super) seen: &'a city::Neighbourhood,
    pub(super) delegates: &'a std::sync::Arc<std::sync::Mutex<collab::DelegateDesk>>,
    pub(super) context: &'a runtime::ContextReading,
}

/// Takes a desk, or says why it cannot be taken.
///
/// The one reading of a poisoned lock in this assembly. Under
/// `panic = "abort"` a thread cannot die holding a desk, so the error
/// arm is unreachable in the shipped binary and still written, because
/// a test build unwinds and a desk left locked there is a fact worth a
/// stable code rather than a second panic (`crates/sprawling/Spec.lean` §8-44).
pub(super) fn held<'a, T>(
    desk: &'a std::sync::Mutex<T>,
    what: &'static str,
) -> Result<std::sync::MutexGuard<'a, T>, AxError> {
    desk.lock().map_err(|_| {
        AxError::failure(
            AxCode::StorageFatal,
            what,
            "the desk was left locked by a thread that died",
        )
        .with_recovery("restart this city")
    })
}

impl Workbench {
    /// Hands the bench to the drive, once.
    ///
    /// # Errors
    /// Refuses a second ask, which is a defect in the caller rather
    /// than a state a run can be in: one dispatch drives once.
    pub(super) fn take_bench(&mut self) -> Result<ToolBench, AxError> {
        self.bench.take().ok_or_else(|| {
            AxError::failure(
                AxCode::ConfigInvalid,
                "carry the bench into the drive",
                "this workbench has already been driven",
            )
            .with_recovery("report this: one dispatch lays out one bench and drives it once")
        })
    }
}

impl Site {
    /// What a checkpoint covers for this run.
    ///
    /// Under review the worktree is this run's alone, so everything that
    /// changed inside it is this run's to offer - the shelf entries it
    /// filed included, which sit at the building rather than in the
    /// room.
    ///
    /// Without a lease the checkpoint is **the run's write domain**, which is
    /// the building's own subtree plus whatever else its `RULES.toml`
    /// declares, as `crates/storage/Spec.lean` §8-18 states. The room would be
    /// narrower than the gate: `city::policy::write_domain` defaults to
    /// the whole building, and City Hall's residents reach every document
    /// under theirs. Anything a run wrote outside a room-sized checkpoint would
    /// be staged by no checkpoint, reach no `changes` answer, and have no
    /// `file_discarded` record that could restore it.
    ///
    /// # Errors
    /// Propagates a building whose declared prefixes its own rules
    /// refuse.
    pub(super) fn checkpoint_scope(&self) -> Result<Vec<String>, AxError> {
        if self.lease.is_some() {
            return Ok(tree_scope(&self.building));
        }
        Ok(self
            .rules
            .write_domain()?
            .prefixes()
            .map(|prefix| prefix.as_str().to_owned())
            .collect())
    }
}

/// What a building under review writes in its tree: its own subtree.
/// The tree is claimed, checkpointed and offered over this one scope, so a
/// claim that checked out less than the checkpoint stages, or an offer that
/// staged more than the claim checked out, cannot happen.
pub(in crate::worker) fn tree_scope(building: &city::Building) -> Vec<String> {
    vec![building.addr().as_str().to_owned()]
}

/// The desks one dispatch lends out, and takes back when the drive ends.
///
/// Grouped because they are lent and taken back together: five handles
/// passed side by side are five chances to take four of them back. Three
/// of them settle in one order in `settle_desks`, `goals` is answered at
/// each call on the accounting thread, and `pr` settles after,
/// once the run has something to show for itself, and it belongs here
/// all the same - what makes them one value is the lending, not the
/// settling.
pub(super) struct Desks {
    pub(super) signals: std::sync::Arc<std::sync::Mutex<collab::SignalDesk>>,
    pub(super) goals: std::sync::Arc<std::sync::Mutex<collab::GoalDesk>>,
    pub(super) plan: std::sync::Arc<std::sync::Mutex<collab::ClaimDesk>>,
    pub(super) shelf: std::sync::Arc<std::sync::Mutex<collab::ArchiveDesk>>,
    pub(super) pr: std::sync::Arc<std::sync::Mutex<collab::PrDesk>>,
    /// The room's workshop, holding what it already got back and handed
    /// down, so a graph laid out again hands nothing down twice.
    pub(super) workshop: std::sync::Arc<std::sync::Mutex<collab::WorkshopDesk>>,
    /// Where the shared plan lives, so the claims that survive are
    /// written back to the file they were checked against.
    pub(super) plan_path: PathBuf,
    /// What was already in the room's queue when it was lent out, which
    /// is what `status` reports as waiting. Read before the queue goes
    /// to the desk, so it is counted here or not at all.
    waiting: u32,
    /// This run's tenure over its room's queue, shown when it lands:
    /// only the holder gives a queue back (`crates/sprawling/Spec.lean` §8-46-9).
    pub(super) tenure: super::QueueTenure,
}

/// The desks a run's bench holds while it drives: clones of the handles
/// in [`Desks`], for the lane that lays the bench out. The room's queue
/// and the tenure over it stay in [`Desks`], with the landing, because
/// only the holder gives a queue back.
pub(in crate::worker) struct BenchDesks {
    pub(in crate::worker) signals: std::sync::Arc<std::sync::Mutex<collab::SignalDesk>>,
    goals: std::sync::Arc<std::sync::Mutex<collab::GoalDesk>>,
    plan: std::sync::Arc<std::sync::Mutex<collab::ClaimDesk>>,
    shelf: std::sync::Arc<std::sync::Mutex<collab::ArchiveDesk>>,
    pr: std::sync::Arc<std::sync::Mutex<collab::PrDesk>>,
    workshop: std::sync::Arc<std::sync::Mutex<collab::WorkshopDesk>>,
    waiting: u32,
}

impl Desks {
    pub(in crate::worker) fn for_bench(&self) -> BenchDesks {
        BenchDesks {
            signals: std::sync::Arc::clone(&self.signals),
            goals: std::sync::Arc::clone(&self.goals),
            plan: std::sync::Arc::clone(&self.plan),
            shelf: std::sync::Arc::clone(&self.shelf),
            pr: std::sync::Arc::clone(&self.pr),
            workshop: std::sync::Arc::clone(&self.workshop),
            waiting: self.waiting,
        }
    }
}

/// What a run can be told about itself at the moment it starts.
///
/// Every field here is read from something, never a constant: the run policy,
/// the write domain the building granted, the budget, the context limit
/// and the locks. City.md tells a model to call `status` for exactly
/// those, and a row of constants would teach a model that obeyed not to
/// ask again.
///
/// The context used and the children are not here: both move while the
/// run goes on, so `status` reads them live. `worktree_disk` is zero
/// because measuring a tree costs a walk of it, and a number nobody has
/// asked for is not worth one.
pub(super) struct Situation<'a> {
    pub(super) addr: &'a Address,
    pub(super) who: &'a str,
    signals_pending: u32,
    policy: kernel::RunPolicy,
    write_domain: &'a kernel::WriteDomain,
    worktree: &'a Path,
    trust: &'a kernel::Autonomy,
    context_tokens: u64,
    locks: Vec<String>,
    neighbours: u32,
}

pub(super) fn status_snapshot(situation: Situation<'_>) -> runtime::StatusSnapshot {
    runtime::StatusSnapshot {
        who: situation.who.to_owned(),
        addr: situation.addr.clone(),
        policy: situation.policy,
        ctx_limit: kernel::Tokens::new(situation.context_tokens),
        trust: autonomy_word::spell(situation.trust),
        write_domain: situation
            .write_domain
            .prefixes()
            .map(|prefix| prefix.as_str().to_owned())
            .collect::<Vec<String>>()
            .join(", "),
        locks: situation.locks,
        worktree_path: situation.worktree.display().to_string(),
        worktree_disk: kernel::ByteLen::default(),
        signals_pending: situation.signals_pending,
        provider_mode: runtime::ProviderMode::Normal,
        neighbours: situation.neighbours,
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
