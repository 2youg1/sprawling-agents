// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The second drive: a run whose turn an official harness takes, in the
//! room's own tree (sprawling-SPEC.md 8-4e, 8-124).
//!
//! It stands beside [`drive_run`](super::lane::drive_run) rather than
//! inside it, because everything that drive is handed - the adapter,
//! the bench, the plan - is a model's. What the two share is the lane
//! they run in, the ledger they write through, the checkpoint gate and
//! the rule that a stopping scope stops the run. The order of the lines
//! is `runtime::run::harness`'s, and the properties it keeps are proved
//! in `crates/agent_protocols/spec/Harness/Session.lean`.

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::process::ChildStdin;
use std::sync::Arc;

use agent_protocols::{
    AcpSession, Answer, Harness, HarnessProcess, Listener, PermissionAsk, Permit, PermitKind,
    StopReason, Update,
};
use kernel::event::Who;
use kernel::event::record::{
    HarnessAnswered, HarnessPermit, HarnessPermitKind, HarnessReported, HarnessStop,
};
use kernel::{Address, AxError, Completion, Ledger, Locator, Payload, RunId, TimeMs};
use runtime::run::{Charter, Conclusion, Cut, HarnessRun};

use super::lane::{DriveContext, scope_stopping};
use crate::worker::workbench::{Lending, Placing, held, lend_tree, tree_scope};
use crate::worker::{Stamping, recording::Notes};

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
    /// `worktree_opened` line (sprawling-SPEC.md 8-41).
    pub(in crate::worker) command: Option<kernel::IdemKey>,
    /// This run's place in the backlog, when somebody handed it down.
    pub(in crate::worker) member: Option<runtime::BacklogId>,
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
    chartered: &Chartered,
) -> storage::Provenance {
    let signed = storage::Provenance::new(
        chartered.run,
        chartered.addr.clone(),
        city,
        storage::ModelChoice {
            id: String::new(),
            effort: None,
        },
    );
    match chartered.predecessor {
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
    let of = harness_provenance(half.city, &half.chartered);
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
            checkpoint_gate: &context.checkpoint_gate,
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
    let turning = Turning {
        run,
        ledger: &mut *ledger,
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

/// What a turn leaves: the run, how the prompt ended, and the first line
/// the turn could not book.
type Turned<'a> = (HarnessRun<'a>, Result<Answer, AxError>, Option<AxError>);

/// Sends the prompt and books what the harness says until it stops.
fn take_the_turn<'a, L: Ledger>(
    half: &HarnessHalf,
    turning: Turning<'a, '_, L>,
    prompting: &mut Prompting,
    context: &DriveContext,
) -> Turned<'a> {
    let clock = &*context.clock;
    let turning = RefCell::new(turning);
    let pending = Cell::new(None);
    let mut halted = || match cut_now(half, context) {
        Some(cut) => {
            pending.set(Some(cut));
            true
        }
        None => false,
    };
    let mut cancelling = || -> Result<(), AxError> {
        let t = clock.now()?;
        let mut held = turning.borrow_mut();
        let Turning { run, ledger, .. } = &mut *held;
        run.cancel(&mut **ledger, pending.get().unwrap_or(Cut::Halt), t)
    };
    let mut report = |update: Update| -> Result<(), AxError> {
        let t = clock.now()?;
        turning.borrow_mut().book(&reported(update), t)
    };
    let mut permit = |ask: &PermissionAsk| -> Permit {
        let chosen = permitted(ask);
        let booked = clock
            .now()
            .and_then(|t| turning.borrow_mut().book_permit(ask, &chosen, t));
        match booked {
            Ok(()) => chosen,
            Err(failed) => {
                turning.borrow_mut().failed.get_or_insert(failed);
                Permit::Cancelled
            }
        }
    };
    let prompted = prompting(
        &half.prompt,
        &mut Listener {
            halted: &mut halted,
            cancelling: &mut cancelling,
            report: &mut report,
            permit: &mut permit,
        },
    );
    let Turning { run, failed, .. } = turning.into_inner();
    (run, prompted, failed)
}

/// The run and the ledger it writes through, shared by the four
/// callbacks a session asks.
struct Turning<'a, 'l, L> {
    run: HarnessRun<'a>,
    ledger: &'l mut L,
    /// The first line a callback that cannot fail could not book.
    failed: Option<AxError>,
}

impl<L: Ledger> Turning<'_, '_, L> {
    fn book(&mut self, reported: &HarnessReported, t: TimeMs) -> Result<(), AxError> {
        self.run.report(&mut *self.ledger, reported, t)
    }

    /// A permission ask and the city's answer to it, one line each.
    fn book_permit(
        &mut self,
        ask: &PermissionAsk,
        chosen: &Permit,
        t: TimeMs,
    ) -> Result<(), AxError> {
        let asked = HarnessReported::PermissionAsked {
            title: ask.title.clone(),
            options: ask
                .options
                .iter()
                .map(|option| HarnessPermit {
                    id: option.id.clone(),
                    name: option.name.clone(),
                    kind: permit_kind(option.kind),
                })
                .collect(),
        };
        self.book(&asked, t)?;
        let answered = HarnessReported::PermissionAnswered {
            chosen: match chosen {
                Permit::Chosen(id) => Some(id.clone()),
                Permit::Cancelled => None,
            },
        };
        self.book(&answered, t)
    }
}

/// Whether the turn is cut now, and why: a stopping scope and a
/// person's cancel are a halt.
///
/// A steer is taken and not delivered: ACP gives a client nothing to
/// send an agent in the middle of a turn, so the line says it went
/// nowhere rather than letting it vanish (sprawling-SPEC.md 8-124).
fn cut_now(half: &HarnessHalf, context: &DriveContext) -> Option<Cut> {
    if scope_stopping(&context.backlog, half.member) {
        return Some(Cut::Halt);
    }
    let asked = context
        .person
        .as_ref()
        .map_or(runtime::Interrupt::None, |ask| ask(half.chartered.run));
    match asked {
        runtime::Interrupt::Cancel => Some(Cut::Halt),
        runtime::Interrupt::None => None,
        runtime::Interrupt::Steer { source, .. } => {
            half.notes.write(
                runtime::diagnostics::Level::Refuse,
                half.staged_at,
                "accounting::worker::driving::harness",
                &format!(
                    "{} is a harness's turn and takes no steer: the one from {source} was not \
                     delivered",
                    half.chartered.run
                ),
            );
            None
        }
    }
}

/// Commits the tree as the harness left it, under the city's one
/// checkpoint at a time.
fn commit(
    root: &Path,
    scope: &[String],
    of: &storage::Provenance,
    context: &DriveContext,
) -> Result<Payload, AxError> {
    let _one_at_a_time = held(&context.checkpoint_gate, "take the checkpoint gate")?;
    storage::Checkpoint::open(root)
        .map_err(storage::StorageError::into_ax)?
        .wave_pre(scope, context.clock.now()?, of)
        .map_err(storage::StorageError::into_ax)
}

/// The city's answer to a permission ask (sprawling-SPEC.md 8-4e rule
/// 9): the first "allow once", else the first "reject once", else
/// cancelled. Never "always": that would decide for later calls.
fn permitted(ask: &PermissionAsk) -> Permit {
    let first = |kind: PermitKind| {
        ask.options
            .iter()
            .find(|option| option.kind == kind)
            .map(|option| Permit::Chosen(option.id.clone()))
    };
    first(PermitKind::AllowOnce)
        .or_else(|| first(PermitKind::RejectOnce))
        .unwrap_or(Permit::Cancelled)
}

/// One report in the city's own words (kernel-SPEC 12.9).
fn reported(update: Update) -> HarnessReported {
    match update {
        Update::Text(text) => HarnessReported::Said { text },
        Update::Thought(text) => HarnessReported::Thought { text },
        Update::ToolCall { id, title, kind } => HarnessReported::ToolCall {
            call: id,
            title,
            kind,
        },
        Update::ToolCallStatus { id, status } => {
            HarnessReported::ToolCallStatus { call: id, status }
        }
        Update::Other { variant } => HarnessReported::Other { variant },
    }
}

fn permit_kind(kind: PermitKind) -> HarnessPermitKind {
    match kind {
        PermitKind::AllowOnce => HarnessPermitKind::AllowOnce,
        PermitKind::AllowAlways => HarnessPermitKind::AllowAlways,
        PermitKind::RejectOnce => HarnessPermitKind::RejectOnce,
        PermitKind::RejectAlways => HarnessPermitKind::RejectAlways,
    }
}

/// The answer in the city's own words.
fn answered(answer: Answer) -> HarnessAnswered {
    HarnessAnswered {
        stop: match answer.stop {
            StopReason::EndTurn => HarnessStop::EndTurn,
            StopReason::MaxTokens => HarnessStop::MaxTokens,
            StopReason::MaxTurnRequests => HarnessStop::MaxTurnRequests,
            StopReason::Refusal => HarnessStop::Refusal,
            StopReason::Cancelled => HarnessStop::Cancelled,
        },
        text: answer.text,
    }
}
