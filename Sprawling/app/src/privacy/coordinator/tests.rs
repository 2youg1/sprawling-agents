// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]

use std::rc::Rc;

use kernel::AxError;
use proptest::prelude::*;

use super::super::state::fixtures::{dword, owner};
use super::super::target::{RawValue, TaskState};
use super::support::*;
use super::*;
use wire::PrivacyFaultCode;

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
    /// model's properties (`Sprawling/app/spec/Privacy.lean` §16).
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
