// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The second drive: a run whose turn an official harness takes, in the
//! room's own tree (`crates/sprawling/Spec.lean` §8-4e, §8-124).
//!
//! It stands beside [`drive_run`](super::lane::drive_run) rather than
//! inside it, because everything that drive is handed - the adapter,
//! the bench, the plan - is a model's. What the two share is the lane
//! they run in, the ledger they write through, the checkpoint gate and
//! the rule that a stopping scope stops the run. The order of the lines
//! is `runtime::run::harness`'s, and the properties it keeps are proved
//! in `crates/agent_protocols/spec/Harness/Session.lean`.

use std::path::{Path, PathBuf};
use std::process::ChildStdin;
use std::sync::Arc;

use agent_protocols::{AcpSession, Answer, Harness, HarnessProcess, Listener};
use kernel::event::Who;
use kernel::{Address, AxError, Completion, Ledger, Locator, Payload, RunId, TimeMs};
use runtime::run::{Charter, Conclusion, HarnessRun};

use super::lane::DriveContext;
use crate::worker::workbench::{Lending, Placing, lend_tree, tree_scope};
use crate::worker::{Stamping, recording::Notes};

mod turn;

use turn::{Turning, answered, take_the_turn};

/// One open session with a harness: a prompt, read to its end.
pub(crate) type Prompting =
    Box<dyn FnMut(&str, &mut Listener<'_>) -> Result<Answer, AxError> + Send>;

/// Starts a harness in a directory and opens a session with it there.
/// A closure rather than a trait: the second implementation is the
/// tests', which play the agent over a pipe.
pub(crate) type StartHarness =
    Arc<dyn Fn(Harness, &Path) -> Result<Prompting, AxError> + Send + Sync>;

/// The harnesses this machine starts: the vendor's own program, in the
/// room's tree, through `agent_protocols::HarnessProcess`.
pub(in crate::worker) fn on_this_machine() -> StartHarness {
    Arc::new(
        |harness: Harness, cwd: &Path| -> Result<Prompting, AxError> {
            let (process, session) = HarnessProcess::start(harness, cwd)?;
            let mut seated = Seated {
                session,
                _process: process,
            };
            Ok(Box::new(move |text: &str, listener: &mut Listener<'_>| {
                seated.prompt(text, listener)
            }))
        },
    )
}

/// A session and the child it speaks to, held together so the child is
/// killed when the session is dropped.
struct Seated {
    session: AcpSession<ChildStdin>,
    _process: HarnessProcess,
}

impl Seated {
    fn prompt(&mut self, text: &str, listener: &mut Listener<'_>) -> Result<Answer, AxError> {
        self.session.prompt(text, listener)
    }
}

/// What `run_started` says about a harness run, owned so the value can
/// leave the accounting thread.
pub(in crate::worker) struct Chartered {
    pub(in crate::worker) run: RunId,
    pub(in crate::worker) who: String,
    pub(in crate::worker) addr: Address,
    pub(in crate::worker) task: String,
    pub(in crate::worker) goal: String,
    pub(in crate::worker) job: Locator,
    pub(in crate::worker) parent: Option<RunId>,
    pub(in crate::worker) predecessor: Option<RunId>,
    pub(in crate::worker) dispatched_by: Who,
    pub(in crate::worker) policy: kernel::RunPolicy,
}

impl Chartered {
    fn charter(&self) -> Charter<'_> {
        Charter {
            run: self.run,
            who: &self.who,
            addr: &self.addr,
            task: &self.task,
            goal: &self.goal,
            job: &self.job,
            parent: self.parent,
            predecessor: self.predecessor,
            dispatched_by: &self.dispatched_by,
            skills: &[],
            policy: self.policy,
            // A harness run is handed no prefix of this city's, so it
            // froze no names (`crates/runtime/Spec.lean` §8-56).
            naming: None,
            // Its first words are the prompt handed to the harness's own
            // session; the city writes it no first message to record
            // (`crates/kernel/Spec.lean` §8-82-1).
            opening: None,
            // Nor does it send this city's requests, so no effort froze.
            effort: None,
        }
    }
}

/// What one harness drive is handed, owned so it can leave the thread
/// that staged it.
pub(in crate::worker) struct HarnessHalf {
    pub(in crate::worker) harness: Harness,
    pub(in crate::worker) start: StartHarness,
    pub(in crate::worker) chartered: Chartered,
    pub(in crate::worker) building: city::Building,
    /// The one prompt this run sends: the room's brief.
    pub(in crate::worker) prompt: String,
    pub(in crate::worker) handoff: runtime::handoff::Handoff,
    pub(in crate::worker) city_root: PathBuf,
    /// The city's genesis line, which signs the tree's commits.
    pub(in crate::worker) city: kernel::B3Hash,
    /// The key of the command this dispatch answers, stamped on the
    /// `worktree_opened` line (`crates/sprawling/Spec.lean` §8-41).
    pub(in crate::worker) command: Option<kernel::IdemKey>,
    /// This run's place in the backlog, when somebody handed it down.
    pub(in crate::worker) member: Option<runtime::BacklogId>,
    /// How long the turn may take, from the moment the run starts: the
    /// building's `harness_minutes` (`crates/sprawling/Spec.lean` §8-124).
    pub(in crate::worker) ceiling_ms: u64,
    pub(in crate::worker) notes: Notes,
    /// Where the ledger stood when the dispatch was staged, which a
    /// diagnostic line written here is anchored at.
    pub(in crate::worker) staged_at: kernel::Seq,
}

/// What a harness drive carries home: who ran, the tree it borrowed, and
/// how the run ended.
pub(crate) struct HarnessDriven {
    pub(in crate::worker) run: RunId,
    pub(in crate::worker) who: String,
    pub(in crate::worker) building: city::Building,
    pub(in crate::worker) city: kernel::B3Hash,
    pub(in crate::worker) member: Option<runtime::BacklogId>,
    /// The room's tree, given back by the landing whatever the outcome.
    pub(in crate::worker) lease: Option<storage::WorktreeLease>,
    pub(in crate::worker) outcome: Result<Completion, AxError>,
}

/// Lends the room its tree, starts the harness in it and drives its one
/// turn.
///
/// Generic in the ledger for the reason [`drive_run`](super::lane::drive_run)
/// is: a lane writes through its relay.
pub(crate) fn drive_harness<L: Ledger>(
    half: HarnessHalf,
    ledger: &mut L,
    context: DriveContext,
) -> HarnessDriven {
    let mut lease = None;
    let outcome = drive_turn(&half, &mut lease, ledger, &context);
    HarnessDriven {
        run: half.chartered.run,
        who: half.chartered.who,
        building: half.building,
        city: half.city,
        member: half.member,
        lease,
        outcome,
    }
}

/// What a commit this run makes is signed with. The city does not know
/// which model the harness used, so the model id is left empty rather
/// than invented (`storage::ModelChoice`).
pub(in crate::worker) fn harness_provenance(
    city: kernel::B3Hash,
    run: RunId,
    addr: &Address,
    predecessor: Option<RunId>,
) -> storage::Provenance {
    let signed = storage::Provenance::new(
        run,
        addr.clone(),
        city,
        storage::ModelChoice {
            id: String::new(),
            effort: None,
        },
    );
    match predecessor {
        Some(predecessor) => signed.succeeding(predecessor),
        None => signed,
    }
}

/// The turn itself: tree, harness, opening, prompt, and the freeze.
///
/// # Errors
/// Propagates a tree that will not be lent, a harness that will not
/// start, and every failure of the run's own lines. A session that ends
/// without a stop reason, a booking that failed during the turn and a
/// tree that will not commit are propagated after the run froze.
fn drive_turn<L: Ledger>(
    half: &HarnessHalf,
    lease: &mut Option<storage::WorktreeLease>,
    ledger: &mut L,
    context: &DriveContext,
) -> Result<Completion, AxError> {
    let clock = &*context.clock;
    let of = harness_provenance(
        half.city,
        half.chartered.run,
        &half.chartered.addr,
        half.chartered.predecessor,
    );
    let tree = lend_tree(
        &Lending {
            addr: &half.chartered.addr,
            building: &half.building,
            run_id: half.chartered.run,
            who: &half.chartered.who,
            of: &of,
        },
        &Placing {
            city_root: &half.city_root,
            city: half.city,
            clock,
        },
        &mut Stamping {
            ledger: &mut *ledger,
            command: half.command,
            clock,
        },
    )?;
    let root = tree.path().to_path_buf();
    *lease = Some(tree);
    let mut prompting = (half.start)(half.harness, &root)?;
    let mut now = || clock.now();
    let run = HarnessRun::open(half.chartered.charter(), &mut *ledger, &mut now)?;
    let deadline = TimeMs::new(clock.now()?.value().saturating_add(half.ceiling_ms));
    let turning = Turning {
        run,
        ledger: &mut *ledger,
        deadline,
        failed: None,
    };
    let (run, prompted, failed) = take_the_turn(half, turning, &mut prompting, context);
    // The session and its child go before the tree is committed: a
    // harness still running could write after the commit.
    drop(prompting);
    let failure = match (prompted, failed) {
        (Ok(answer), None) => {
            let scope = tree_scope(&half.building);
            match commit(&root, &scope, &of, context) {
                Ok(committed) => {
                    return run.conclude(
                        &mut *ledger,
                        Conclusion {
                            committed,
                            answered: answered(answer),
                            handoff: &half.handoff,
                        },
                        clock.now()?,
                    );
                }
                Err(uncommitted) => uncommitted,
            }
        }
        // A line the turn could not book outranks what the session said:
        // the history is the thing that failed.
        (Ok(_) | Err(_), Some(booking)) => booking,
        (Err(session), None) => session,
    };
    run.abandon(&mut *ledger, &half.handoff, clock.now()?)?;
    Err(failure)
}

/// Commits the tree as the harness left it, on this run's own index
/// (`crates/sprawling/spec/Accounting/Views.lean` §8-46-13).
fn commit(
    root: &Path,
    scope: &[String],
    of: &storage::Provenance,
    context: &DriveContext,
) -> Result<Payload, AxError> {
    let mut own =
        storage::Checkpoint::open_writer(root, of.run()).map_err(storage::StorageError::into_ax)?;
    let committed = own
        .wave_pre(scope, context.clock.now()?, of)
        .map_err(storage::StorageError::into_ax)?;
    own.close_writer().map_err(storage::StorageError::into_ax)?;
    Ok(committed)
}
