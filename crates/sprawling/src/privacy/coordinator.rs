// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The one order of a privacy operation (`crates/sprawling/spec/Privacy.lean`
//! §9): identity, the locked history, a fresh read, a durable `Prepared`,
//! the write, the readback, a rollback when the readback is a third value,
//! and the receipt. What to do at each step is decided by
//! [`super::plan`]; this module only carries it out, through two ports.

use std::num::NonZeroU64;

use kernel::{AxError, SecretRef};
use wire::{PrivacyControl, PrivacySettlement};

use super::fault::{HistoryFault, PrivacyFault, ReadFault, Unconfirmed, WriteFault};
use super::journal::LockedJournal;
use super::plan::{self, ApplyPlan, ReconcilePlan, Request, RestorePlan, RollbackEnd, Verdict};
use super::state::{Event, History, Intent, Line, Outcome, SCHEMA};
use super::target::{Reading, Snapshot};

/// How long after a command is accepted its write may still start
/// (`crates/sprawling/spec/Privacy.lean` §14).
const TTL_MS: u64 = 60_000;

/// The machine a privacy operation runs on: its clock, the identity of
/// the account running it, and the targets of the controls.
pub(crate) trait Host: accounting::Clock {
    type Identity;

    /// Samples the identity of the account this process runs as.
    ///
    /// # Errors
    /// The identity could not be read.
    fn identity(&mut self) -> Result<Self::Identity, AxError>;

    /// The owner reference new intents are prepared under: `recorded` once
    /// it is bound to `identity`, or, for a history with no owner yet, a
    /// binding made for `identity`.
    ///
    /// # Errors
    /// `recorded` belongs to another identity, or the binding could not be
    /// read or made.
    fn owner(
        &mut self,
        recorded: Option<&SecretRef>,
        identity: &Self::Identity,
    ) -> Result<SecretRef, AxError>;

    /// Reads the target of `control`.
    ///
    /// # Errors
    /// [`ReadFault`].
    fn read(&mut self, control: PrivacyControl) -> Result<Reading, ReadFault>;

    /// Writes `value` to the target of `control`. A reported failure never
    /// decides the outcome: the readback does.
    ///
    /// # Errors
    /// [`WriteFault`].
    fn write(&mut self, control: PrivacyControl, value: &Snapshot) -> Result<(), WriteFault>;
}

/// The history an operation runs against, held for the whole operation.
pub(super) trait Journal {
    fn history(&self) -> &History;

    /// Appends `line` and returns once it is on disk.
    ///
    /// # Errors
    /// The line was refused or could not be made durable.
    fn append_durable(&mut self, line: &Line) -> Result<(), HistoryFault>;
}

impl Journal for LockedJournal {
    fn history(&self) -> &History {
        Self::history(self)
    }

    fn append_durable(&mut self, line: &Line) -> Result<(), HistoryFault> {
        Self::append_durable(self, line)
    }
}

/// What the person asked for. `expected` is the snapshot the page showed
/// when the person confirmed (Privacy.Confirmation D58).
pub(super) enum Command {
    Apply {
        control: PrivacyControl,
        expected: Snapshot,
    },
    Restore {
        control: PrivacyControl,
        expected: Snapshot,
    },
    Reconcile {
        expected: Snapshot,
    },
}

/// A command that ended well, as the CLI prints it.
#[derive(Debug, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "done", rename_all = "snake_case")]
pub(super) enum Done {
    Applied {
        operation: NonZeroU64,
    },
    Restored {
        operation: NonZeroU64,
    },
    AlreadyWritten,
    Reconciled {
        operation: NonZeroU64,
        settlement: PrivacySettlement,
    },
}

/// Runs `command` against the history `open` locks, through `host`.
///
/// The identity is sampled before the history is opened, and the history
/// releases nothing until its owner reference passed the identity check.
///
/// # Errors
/// [`PrivacyFault`]; whatever reached the history before the fault stays
/// there, and a fault after a durable `Prepared` is itself recorded.
pub(super) fn run<H: Host, J: Journal>(
    host: &mut H,
    open: impl FnOnce() -> Result<J, HistoryFault>,
    command: &Command,
) -> Result<Done, PrivacyFault> {
    let accepted = host.now().map_err(PrivacyFault::Clock)?;
    let identity = host.identity().map_err(PrivacyFault::Identity)?;
    let mut session = Session {
        journal: open().map_err(PrivacyFault::History)?,
        host,
        identity,
        deadline: accepted.value().saturating_add(TTL_MS),
    };
    match command {
        Command::Apply { control, expected } => session.change(Action::Apply, *control, expected),
        Command::Restore { control, expected } => {
            session.change(Action::Restore, *control, expected)
        }
        Command::Reconcile { expected } => session.reconcile(expected),
    }
}

#[derive(Clone, Copy)]
enum Action {
    Apply,
    Restore,
}

/// One operation in progress: the host, the locked history and the
/// identity it was opened for.
struct Session<'h, H: Host, J> {
    host: &'h mut H,
    journal: J,
    identity: H::Identity,
    deadline: u64,
}

impl<H: Host, J: Journal> Session<'_, H, J> {
    fn change(
        &mut self,
        action: Action,
        control: PrivacyControl,
        expected: &Snapshot,
    ) -> Result<Done, PrivacyFault> {
        let (host, identity) = (&mut *self.host, &self.identity);
        let holdings = self
            .journal
            .history()
            .holdings(|recorded| host.owner(recorded, identity))
            .map_err(PrivacyFault::Identity)?;
        let request = Request {
            control,
            operation: holdings.next_operation().ok_or(PrivacyFault::HistoryFull)?,
            expected,
        };
        let fresh = host
            .read(control)
            .map_err(|fault| PrivacyFault::Unreadable { control, fault })?;
        let intent = match action {
            Action::Apply => match plan::plan_apply(&holdings, &request, &fresh) {
                ApplyPlan::Write(intent) => intent,
                ApplyPlan::Unresolved => return Err(PrivacyFault::Unresolved),
                ApplyPlan::Changed => return Err(PrivacyFault::Changed { control }),
                ApplyPlan::TargetAbsent => return Err(PrivacyFault::TargetAbsent { control }),
                ApplyPlan::AlreadyWritten => return Ok(Done::AlreadyWritten),
            },
            Action::Restore => match plan::plan_restore(&holdings, &request, &fresh) {
                RestorePlan::Write(intent) => intent,
                RestorePlan::Unresolved => return Err(PrivacyFault::Unresolved),
                RestorePlan::NothingOwned => return Err(PrivacyFault::NothingOwned { control }),
                RestorePlan::Changed => return Err(PrivacyFault::Changed { control }),
                RestorePlan::Conflict => return Err(PrivacyFault::Conflict { control }),
            },
        };
        self.carry_out(&intent)
    }

    /// From a planned intent to its receipt: `durablePrepared`, `lapsed`,
    /// `writeStarted`, then the readback's verdict.
    fn carry_out(&mut self, intent: &Intent) -> Result<Done, PrivacyFault> {
        let (control, operation) = (intent.control, intent.operation);
        self.unexpired(control, None)?;
        self.journal
            .append_durable(&line(Event::Prepared {
                intent: intent.clone(),
            }))
            .map_err(PrivacyFault::History)?;
        if let Err(fault) = self.unexpired(control, Some(operation)) {
            self.record(intent, Outcome::NotApplied)?;
            return Err(fault);
        }
        let written = self.host.write(control, &intent.modified).err();
        let readback = match self.host.read(control) {
            Ok(reading) => reading.value,
            Err(fault) => {
                return self.lose(intent, Unconfirmed::ReadbackUnreadable(fault));
            }
        };
        match plan::settle(intent, plan::judge_readback(intent, &readback)) {
            Verdict::Applied => self
                .record(intent, Outcome::Applied)
                .map(|()| Done::Applied { operation }),
            Verdict::Restored => self
                .record(intent, Outcome::Restored)
                .map(|()| Done::Restored { operation }),
            Verdict::NotApplied => {
                self.record(intent, Outcome::NotApplied)?;
                Err(PrivacyFault::NotApplied {
                    control,
                    operation,
                    cause: written,
                })
            }
            Verdict::RollBack => self.roll_back(intent),
        }
    }

    /// `rollbackStarted`: the original goes back whatever the deadline,
    /// and only a read of the original ends the rollback.
    fn roll_back(&mut self, intent: &Intent) -> Result<Done, PrivacyFault> {
        let (control, operation) = (intent.control, intent.operation);
        let written = self.host.write(control, &intent.original).err();
        let end = match self.host.read(control) {
            Ok(reading) => plan::settle_rollback(intent, &reading.value),
            Err(fault) => {
                return self.lose(intent, Unconfirmed::RollbackUnreadable(fault));
            }
        };
        match end {
            RollbackEnd::RolledBack => {
                self.record(intent, Outcome::RolledBack)?;
                Err(PrivacyFault::RolledBack { control, operation })
            }
            RollbackEnd::Lost => self.lose(intent, Unconfirmed::RollbackMissed(written)),
        }
    }

    fn lose(&mut self, intent: &Intent, why: Unconfirmed) -> Result<Done, PrivacyFault> {
        self.record(intent, Outcome::Unknown)?;
        Err(PrivacyFault::Unknown {
            control: intent.control,
            operation: intent.operation,
            why,
        })
    }

    fn reconcile(&mut self, expected: &Snapshot) -> Result<Done, PrivacyFault> {
        let (host, identity) = (&mut *self.host, &self.identity);
        let unresolved = self
            .journal
            .history()
            .holdings(|recorded| host.owner(recorded, identity))
            .map_err(PrivacyFault::Identity)?
            .unresolved()
            .cloned()
            .ok_or(PrivacyFault::NothingUnresolved)?;
        let control = unresolved.control;
        let fresh = host
            .read(control)
            .map_err(|fault| PrivacyFault::Unreadable { control, fault })?;
        let settlement = match plan::plan_reconcile(&unresolved, expected, &fresh.value) {
            ReconcilePlan::Settle(settlement) => settlement,
            ReconcilePlan::Changed => return Err(PrivacyFault::Changed { control }),
        };
        let operation = unresolved.operation;
        self.journal
            .append_durable(&line(Event::Reconciled {
                operation,
                settlement,
            }))
            .map_err(PrivacyFault::History)?;
        Ok(Done::Reconciled {
            operation,
            settlement,
        })
    }

    /// The model's `now < expires`.
    fn unexpired(
        &self,
        control: PrivacyControl,
        operation: Option<NonZeroU64>,
    ) -> Result<(), PrivacyFault> {
        let now = self.host.now().map_err(PrivacyFault::Clock)?;
        if now.value() < self.deadline {
            Ok(())
        } else {
            Err(PrivacyFault::Expired { control, operation })
        }
    }

    fn record(&mut self, intent: &Intent, outcome: Outcome) -> Result<(), PrivacyFault> {
        self.journal
            .append_durable(&line(Event::Finished {
                operation: intent.operation,
                outcome,
            }))
            .map_err(|fault| PrivacyFault::ReceiptLost {
                control: intent.control,
                operation: intent.operation,
                fault,
            })
    }
}

fn line(event: Event) -> Line {
    Line {
        schema: SCHEMA,
        event,
    }
}

impl Command {
    /// What a person reads this command as, in an error.
    pub(super) const fn action(&self) -> &'static str {
        match self {
            Self::Apply { .. } => "apply privacy control",
            Self::Restore { .. } => "restore privacy control",
            Self::Reconcile { .. } => "reconcile privacy operation",
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests {
    use std::cell::RefCell;
    use std::collections::{BTreeMap, VecDeque};
    use std::rc::Rc;

    use kernel::{AxCode, TimeMs};
    use proptest::prelude::*;

    use super::super::state::fixtures::{dword, owner};
    use super::super::target::{RawValue, TaskState};
    use super::*;
    use wire::PrivacyFaultCode;

    /// One control of each operation kind.
    const CONTROLS: [PrivacyControl; 4] = [
        PrivacyControl::FeedbackNotifications,
        PrivacyControl::StartLaunchTracking,
        PrivacyControl::PowershellTelemetryOptout,
        PrivacyControl::DeviceCensusTask,
    ];

    fn absent() -> Snapshot {
        Snapshot::Registry(RawValue::Absent)
    }

    fn task(state: fn([u8; 32]) -> TaskState, digest: u8) -> Snapshot {
        Snapshot::Task(state([digest; 32]))
    }

    fn enabled(definition_sha256: [u8; 32]) -> TaskState {
        TaskState::Enabled { definition_sha256 }
    }

    fn disabled(definition_sha256: [u8; 32]) -> TaskState {
        TaskState::Disabled { definition_sha256 }
    }

    /// A value of the same kind that nobody asked for.
    fn third(value: &Snapshot) -> Snapshot {
        match value {
            Snapshot::Registry(_) => dword(77),
            Snapshot::Task(_) => task(disabled, 9),
        }
    }

    #[derive(Debug, Clone, Copy)]
    enum WriteEffect {
        Lands,
        LandsReportingFailure,
        LandsThird,
        Refused(Refusal),
    }

    #[derive(Debug, Clone, Copy)]
    enum Refusal {
        Denied,
        Declined,
        Failed,
    }

    #[derive(Debug, Clone, Copy)]
    enum ReadEffect {
        Reads,
        Denied,
        Fails,
    }

    #[derive(Debug, Clone, Copy)]
    enum AppendEffect {
        Durable,
        Fails,
    }

    /// What the host and the history saw, in order.
    #[derive(Debug, Clone)]
    enum Seen {
        Run,
        Read,
        Wrote {
            control: PrivacyControl,
            value: Snapshot,
        },
        Appended {
            line: Line,
            values: BTreeMap<PrivacyControl, Snapshot>,
        },
    }

    /// The host's true values, the history's lines, the faults the next
    /// calls meet, and the log of everything seen.
    struct World {
        now: u64,
        foreign: bool,
        values: BTreeMap<PrivacyControl, Snapshot>,
        lines: Vec<Line>,
        reads: VecDeque<ReadEffect>,
        writes: VecDeque<WriteEffect>,
        appends: VecDeque<AppendEffect>,
        lapse_on_prepared: bool,
        log: Vec<Seen>,
    }

    type Shared = Rc<RefCell<World>>;

    fn world() -> Shared {
        Rc::new(RefCell::new(World {
            now: 1_000,
            foreign: false,
            values: BTreeMap::from([
                (CONTROLS[0], absent()),
                (CONTROLS[1], dword(1)),
                (CONTROLS[2], absent()),
                (CONTROLS[3], task(enabled, 1)),
            ]),
            lines: Vec::new(),
            reads: VecDeque::new(),
            writes: VecDeque::new(),
            appends: VecDeque::new(),
            lapse_on_prepared: false,
            log: Vec::new(),
        }))
    }

    struct FaultHost(Shared);

    impl accounting::Clock for FaultHost {
        fn now(&self) -> Result<TimeMs, AxError> {
            Ok(TimeMs::new(self.0.borrow().now))
        }
    }

    fn failed(what: &str) -> AxError {
        AxError::failure(AxCode::ToolUnavailable, "fixture host", what)
            .with_recovery("fixture fault")
    }

    impl Host for FaultHost {
        type Identity = ();

        fn identity(&mut self) -> Result<(), AxError> {
            Ok(())
        }

        fn owner(&mut self, recorded: Option<&SecretRef>, (): &()) -> Result<SecretRef, AxError> {
            match (recorded, self.0.borrow().foreign) {
                (None, false) => Ok(owner()),
                (Some(recorded), false) if *recorded == owner() => Ok(recorded.clone()),
                (None | Some(_), true | false) => Err(failed("another identity")),
            }
        }

        fn read(&mut self, control: PrivacyControl) -> Result<Reading, ReadFault> {
            let mut world = self.0.borrow_mut();
            world.log.push(Seen::Read);
            match world.reads.pop_front().unwrap_or(ReadEffect::Reads) {
                ReadEffect::Reads => Ok(Reading {
                    value: world.values[&control].clone(),
                    key_existed: true,
                }),
                ReadEffect::Denied => Err(ReadFault::AccessDenied),
                ReadEffect::Fails => Err(ReadFault::Failed(failed("read"))),
            }
        }

        fn write(&mut self, control: PrivacyControl, value: &Snapshot) -> Result<(), WriteFault> {
            let mut world = self.0.borrow_mut();
            world.log.push(Seen::Wrote {
                control,
                value: value.clone(),
            });
            let effect = world.writes.pop_front().unwrap_or(WriteEffect::Lands);
            let landed = match effect {
                WriteEffect::Lands | WriteEffect::LandsReportingFailure => value.clone(),
                WriteEffect::LandsThird => third(value),
                WriteEffect::Refused(_) => world.values[&control].clone(),
            };
            world.values.insert(control, landed);
            match effect {
                WriteEffect::Lands | WriteEffect::LandsThird => Ok(()),
                WriteEffect::LandsReportingFailure | WriteEffect::Refused(Refusal::Failed) => {
                    Err(WriteFault::Failed(failed("write")))
                }
                WriteEffect::Refused(Refusal::Denied) => Err(WriteFault::AccessDenied),
                WriteEffect::Refused(Refusal::Declined) => Err(WriteFault::Declined),
            }
        }
    }

    /// The history kept in memory and judged by the same fold a file is.
    struct MemoryJournal {
        world: Shared,
        history: History,
    }

    impl MemoryJournal {
        fn open(world: &Shared) -> Result<Self, HistoryFault> {
            let history = History::fold(world.borrow().lines.clone())?;
            Ok(Self {
                world: Rc::clone(world),
                history,
            })
        }
    }

    impl Journal for MemoryJournal {
        fn history(&self) -> &History {
            &self.history
        }

        fn append_durable(&mut self, line: &Line) -> Result<(), HistoryFault> {
            let mut world = self.world.borrow_mut();
            if let Some(AppendEffect::Fails) = world.appends.pop_front() {
                return Err(HistoryFault::Io(std::io::Error::other("fixture sync")));
            }
            let mut lines = world.lines.clone();
            lines.push(line.clone());
            self.history = History::fold(lines.clone())?;
            world.lines = lines;
            if world.lapse_on_prepared && matches!(line.event, Event::Prepared { .. }) {
                world.now += TTL_MS;
            }
            let values = world.values.clone();
            world.log.push(Seen::Appended {
                line: line.clone(),
                values,
            });
            Ok(())
        }
    }

    #[derive(Debug, Clone, Copy)]
    enum Ask {
        Apply,
        Restore,
        Reconcile,
    }

    /// One command with the faults its calls meet, after an optional
    /// change made outside this app.
    #[derive(Debug, Clone)]
    struct Turn {
        external: Option<(usize, usize)>,
        ask: Ask,
        control: usize,
        stale: bool,
        reads: [ReadEffect; 3],
        writes: [WriteEffect; 2],
        appends: [AppendEffect; 2],
        lapse: bool,
    }

    fn outside(control: PrivacyControl, choice: usize) -> Snapshot {
        let choices = if control == PrivacyControl::DeviceCensusTask {
            [
                task(enabled, 1),
                task(disabled, 1),
                Snapshot::Task(TaskState::Absent),
                task(enabled, 2),
            ]
        } else {
            [absent(), dword(1), dword(0), dword(5)]
        };
        choices[choice % 4].clone()
    }

    fn stale(current: &Snapshot) -> Snapshot {
        match current {
            Snapshot::Registry(RawValue::Absent) => dword(42),
            Snapshot::Registry(RawValue::Present { .. }) => absent(),
            Snapshot::Task(TaskState::Absent) => task(enabled, 3),
            Snapshot::Task(TaskState::Enabled { .. } | TaskState::Disabled { .. }) => {
                Snapshot::Task(TaskState::Absent)
            }
        }
    }

    /// The control of the operation the history leaves open, if any.
    fn open_control(world: &World) -> Option<PrivacyControl> {
        let history = History::fold(world.lines.clone()).ok()?;
        let holdings = history.holdings(|_| Ok(owner())).ok()?;
        holdings.unresolved().map(|intent| intent.control)
    }

    fn command(shared: &Shared, turn: &Turn) -> Command {
        let mut world = shared.borrow_mut();
        if let Some((index, choice)) = turn.external {
            let control = CONTROLS[index % 4];
            world.values.insert(control, outside(control, choice));
        }
        let control = match turn.ask {
            Ask::Reconcile => open_control(&world).unwrap_or(CONTROLS[turn.control]),
            Ask::Apply | Ask::Restore => CONTROLS[turn.control],
        };
        let current = world.values[&control].clone();
        let expected = if turn.stale { stale(&current) } else { current };
        world.reads = turn.reads.into();
        world.writes = turn.writes.into();
        world.appends = turn.appends.into();
        world.lapse_on_prepared = turn.lapse;
        world.log.push(Seen::Run);
        match turn.ask {
            Ask::Apply => Command::Apply { control, expected },
            Ask::Restore => Command::Restore { control, expected },
            Ask::Reconcile => Command::Reconcile { expected },
        }
    }

    /// Runs one turn and maps its fault to the error a person reads; the
    /// stable code leads the subject of every error this module words.
    fn play(shared: &Shared, turn: &Turn) -> Result<Done, AxError> {
        let command = command(shared, turn);
        run(
            &mut FaultHost(Rc::clone(shared)),
            || MemoryJournal::open(shared),
            &command,
        )
        .map_err(|fault| {
            let code = fault.code();
            let error = fault.into_ax(command.action());
            let worded = !matches!(
                code,
                PrivacyFaultCode::Identity | PrivacyFaultCode::Clock | PrivacyFaultCode::History
            );
            assert!(
                !worded || error.subject().starts_with(code.as_str()),
                "{error}"
            );
            error
        })
    }

    /// The model's properties, read off the log of one run of turns:
    /// `reachable_durable` (every host write follows a durable `Prepared`
    /// of its control and writes its modified value, or its original once
    /// the modified was written), `move_owns_only_matched` and the readback
    /// rule (a receipt states what the host truly held), `step_rollback_ends`
    /// (`RolledBack` only over the original) and `trace_unknown` (nothing is
    /// written or prepared between an unresolved operation and the person's
    /// check of it).
    fn check(log: &[Seen]) {
        let mut open: Option<Intent> = None;
        let mut blocked = false;
        let mut wrote_modified = false;
        for seen in log {
            match seen {
                Seen::Run => blocked = blocked || open.is_some(),
                Seen::Read => (),
                Seen::Wrote { control, value } => {
                    assert!(!blocked, "a write while an operation is unresolved");
                    let intent = open.as_ref().expect("a write with no durable Prepared");
                    assert_eq!(*control, intent.control);
                    if *value == intent.modified {
                        wrote_modified = true;
                    } else {
                        assert!(wrote_modified && *value == intent.original);
                    }
                }
                Seen::Appended { line, values } => match &line.event {
                    Event::Prepared { intent } => {
                        assert!(open.is_none() && !blocked);
                        open = Some(intent.clone());
                        wrote_modified = false;
                    }
                    Event::Finished { operation, outcome } => {
                        let intent = open.clone().expect("a receipt with no Prepared");
                        assert_eq!(*operation, intent.operation);
                        let truth = &values[&intent.control];
                        match outcome {
                            Outcome::Applied | Outcome::Restored => {
                                assert_eq!(*truth, intent.modified);
                            }
                            Outcome::NotApplied | Outcome::RolledBack => {
                                assert_eq!(*truth, intent.original, "{outcome:?}");
                            }
                            Outcome::Unknown => {
                                blocked = true;
                                continue;
                            }
                        }
                        open = None;
                    }
                    Event::Reconciled {
                        operation,
                        settlement,
                    } => {
                        let intent = open.take().expect("a check with nothing open");
                        assert_eq!(*operation, intent.operation);
                        let truth = &values[&intent.control];
                        match settlement {
                            PrivacySettlement::Applied | PrivacySettlement::Restored => {
                                assert_eq!(*truth, intent.modified);
                            }
                            PrivacySettlement::NotApplied => assert_eq!(*truth, intent.original),
                            PrivacySettlement::Abandoned => {
                                assert!(*truth != intent.modified && *truth != intent.original);
                            }
                        }
                        blocked = false;
                    }
                },
            }
        }
    }

    fn turns() -> impl Strategy<Value = Turn> {
        let read = prop_oneof![
            8 => Just(ReadEffect::Reads),
            1 => Just(ReadEffect::Denied),
            1 => Just(ReadEffect::Fails),
        ];
        let write = prop_oneof![
            4 => Just(WriteEffect::Lands),
            1 => Just(WriteEffect::LandsReportingFailure),
            2 => Just(WriteEffect::LandsThird),
            1 => Just(WriteEffect::Refused(Refusal::Denied)),
            1 => Just(WriteEffect::Refused(Refusal::Declined)),
            1 => Just(WriteEffect::Refused(Refusal::Failed)),
        ];
        let append = prop_oneof![8 => Just(AppendEffect::Durable), 1 => Just(AppendEffect::Fails)];
        let ask = prop_oneof![
            3 => Just(Ask::Apply),
            2 => Just(Ask::Restore),
            1 => Just(Ask::Reconcile),
        ];
        (
            proptest::option::weighted(0.3, (0..4usize, 0..4usize)),
            ask,
            0..4usize,
            proptest::bool::weighted(0.2),
            [read.clone(), read.clone(), read],
            [write.clone(), write],
            [append.clone(), append],
            proptest::bool::weighted(0.1),
        )
            .prop_map(
                |(external, ask, control, stale, reads, writes, appends, lapse)| Turn {
                    external,
                    ask,
                    control,
                    stale,
                    reads,
                    writes,
                    appends,
                    lapse,
                },
            )
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(512))]

        /// Any sequence of commands, faults and outside changes keeps the
        /// model's properties (`crates/sprawling/spec/Privacy.lean` §16).
        #[test]
        fn any_fault_sequence_keeps_the_models_properties(
            turns in proptest::collection::vec(turns(), 1..24),
        ) {
            let shared = world();
            for turn in &turns {
                // Every result is judged through the log below.
                drop(play(&shared, turn));
            }
            check(&shared.borrow().log);
        }
    }

    fn calm(ask: Ask, control: usize) -> Turn {
        Turn {
            external: None,
            ask,
            control,
            stale: false,
            reads: [ReadEffect::Reads; 3],
            writes: [WriteEffect::Lands; 2],
            appends: [AppendEffect::Durable; 2],
            lapse: false,
        }
    }

    /// A history that belongs to another identity releases nothing: the
    /// host is neither read nor written and nothing is recorded.
    #[test]
    fn a_foreign_identity_reads_and_writes_nothing() {
        let shared = world();
        assert!(matches!(
            play(&shared, &calm(Ask::Apply, 1)),
            Ok(Done::Applied { .. })
        ));
        let before = shared.borrow().lines.len();
        shared.borrow_mut().foreign = true;
        shared.borrow_mut().log.clear();
        assert!(play(&shared, &calm(Ask::Restore, 1)).is_err());
        assert!(matches!(shared.borrow().log.as_slice(), [Seen::Run]));
        assert_eq!(shared.borrow().lines.len(), before);
    }

    /// The production journal: an apply and its restore leave two
    /// concluded operations on disk and the host as it was.
    #[test]
    fn apply_and_restore_through_the_locked_journal() {
        let dir = tempfile::tempdir().unwrap();
        let path = accounting::home::Home::at(dir.path()).privacy_history();
        let shared = world();
        let control = PrivacyControl::StartLaunchTracking;
        let mut host = FaultHost(Rc::clone(&shared));
        let mut ask = |command| run(&mut host, || LockedJournal::open(&path), &command).unwrap();
        let applied = ask(Command::Apply {
            control,
            expected: dword(1),
        });
        let restored = ask(Command::Restore {
            control,
            expected: dword(0),
        });
        assert_eq!(
            [applied, restored],
            [
                Done::Applied {
                    operation: NonZeroU64::MIN
                },
                Done::Restored {
                    operation: NonZeroU64::MIN.saturating_add(1)
                },
            ]
        );
        assert_eq!(shared.borrow().values[&control], dword(1));
        let history = super::super::journal::read(&path).unwrap();
        assert_eq!(
            serde_json::to_value(history.disclose(|_| Ok(())).unwrap()).unwrap(),
            serde_json::json!([
                { "operation": 1, "outcome": { "finished": "applied" } },
                { "operation": 2, "outcome": { "finished": "restored" } }
            ])
        );
    }
}
