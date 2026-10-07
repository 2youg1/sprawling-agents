// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Serving the privacy page (`crates/sprawling/spec/Privacy/Service.lean`):
//! the answer to `Query::Privacy`, read off the host with the history
//! disclosed only to the account it belongs to, and the operations
//! `Command::PrivacyOperation` asks for, carried out one at a time with
//! their results kept under the page's `idem` (Privacy D69).

use std::collections::{BTreeSet, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use kernel::{AxCode, AxError, IdemKey};
use wire::{
    PrivacyAction, PrivacyAnswer, PrivacyHistory, PrivacyHost, PrivacyOutcome, PrivacyResult,
};

use super::coordinator::{self, Command, Done, Host};
use super::journal::{self, LockedJournal};
use super::target::Snapshot;

/// How many results an answer carries; older ones give way (Privacy D69).
const KEPT: usize = 64;

/// The machine the page reads: the coordinator's port, and the facts its
/// version record states.
pub(crate) trait Machine: Host + Send {
    fn facts(&mut self) -> PrivacyHost;
}

/// The privacy page of one served city.
pub(crate) struct Service<M> {
    machine: Box<dyn Fn() -> M + Send + Sync>,
    history: PathBuf,
    kept: Mutex<Kept>,
    /// Held for the whole of one operation, so operations run one after
    /// another in the order they reached it.
    turn: Mutex<()>,
}

/// The results an answer carries, and every `idem` ever accepted.
#[derive(Default)]
struct Kept {
    outcomes: VecDeque<PrivacyOutcome>,
    accepted: BTreeSet<IdemKey>,
}

/// An accepted operation that has not run yet.
pub(super) struct Operation {
    idem: IdemKey,
    command: Command,
}

impl Service<super::system::System> {
    /// The page of the machine this process runs on, over the person's
    /// own privacy history.
    ///
    /// # Errors
    /// The person's home could not be found.
    pub(crate) fn of_this_machine() -> Result<Self, AxError> {
        Ok(Self::new(
            super::system::this_machine,
            accounting::home::Home::detect()?.privacy_history(),
        ))
    }
}

impl<M: Machine + 'static> Service<M> {
    /// A page served from the history at `history`, through a machine
    /// `machine` makes for each answer and each operation.
    pub(super) fn new(machine: impl Fn() -> M + Send + Sync + 'static, history: PathBuf) -> Self {
        Self {
            machine: Box::new(machine),
            history,
            kept: Mutex::default(),
            turn: Mutex::default(),
        }
    }

    /// Reads the host and answers the page. Every failure of a read is a
    /// part of the answer, so the page always learns what was read.
    ///
    /// The results are copied before the history is read (Privacy.Service
    /// D71): an operation records its conclusion before its result settles,
    /// so a concluded result never outruns the history beside it.
    pub(crate) fn answer(&self) -> PrivacyAnswer {
        let outcomes = self.kept().outcomes.iter().cloned().collect();
        let mut machine = (self.machine)();
        let host = machine.facts();
        let history = self.disclosed(&mut machine);
        let windows = !matches!(host, PrivacyHost::NotWindows);
        super::answer::answer(
            host,
            |control| windows.then(|| machine.read(control).map(|reading| reading.value)),
            history,
            outcomes,
        )
    }

    /// Accepts `action` under `idem` and carries it out on a thread of its
    /// own, after every operation accepted before it.
    ///
    /// # Errors
    /// `expected` is not the host's spelling of a value, an `idem` whose
    /// result already gave way arrives again, or the thread could not be
    /// started; nothing is carried out.
    pub(crate) fn carry_out(
        self: &Arc<Self>,
        action: PrivacyAction,
        idem: IdemKey,
    ) -> Result<(), AxError> {
        let Some(operation) = self.accept(action, idem)? else {
            return Ok(());
        };
        let service = Arc::clone(self);
        std::thread::Builder::new()
            .name("sprawling-privacy".to_owned())
            .spawn(move || service.perform(operation))
            .map(drop)
            .map_err(|source| {
                self.kept().forget(idem);
                AxError::failure(
                    AxCode::ToolUnavailable,
                    "carry out a privacy operation",
                    format!("no thread could be started for it: {source}"),
                )
                .with_recovery("nothing was written; send the operation again")
            })
    }

    /// Records `action` as running under `idem` and hands it back to be
    /// performed; an `idem` accepted before is not accepted again.
    ///
    /// # Errors
    /// `expected` is not the host's spelling of a value, or `idem` was
    /// accepted before and its result already gave way.
    pub(super) fn accept(
        &self,
        action: PrivacyAction,
        idem: IdemKey,
    ) -> Result<Option<Operation>, AxError> {
        let command = command(&action)?;
        let mut kept = self.kept();
        if !kept.accepted.insert(idem) {
            return if kept.outcomes.iter().any(|outcome| outcome.idem == idem) {
                Ok(None)
            } else {
                Err(AxError::failure(
                    AxCode::InvalidArgs,
                    command.action(),
                    "this operation was carried out before and its result is no longer kept",
                )
                .with_recovery(
                    "read the privacy page again; send a new operation to change anything",
                ))
            };
        }
        kept.record(PrivacyOutcome {
            idem,
            action,
            result: PrivacyResult::Running,
        });
        Ok(Some(Operation { idem, command }))
    }

    /// Runs `operation` through the coordinator, after every operation
    /// accepted before it, and keeps its result.
    pub(super) fn perform(&self, operation: Operation) {
        let result = {
            // The gate guards no data: an earlier holder that stopped
            // halfway left nothing here to be inconsistent.
            let _turn = self.turn.lock().unwrap_or_else(PoisonError::into_inner);
            let mut machine = (self.machine)();
            match coordinator::run(
                &mut machine,
                || LockedJournal::open(&self.history),
                &operation.command,
            ) {
                Ok(done) => result(done),
                Err(fault) => PrivacyResult::Refused {
                    code: fault.code(),
                    error: fault.into_ax(operation.command.action()),
                },
            }
        };
        self.kept().settle(operation.idem, result);
    }

    /// The history as this account may see it (Privacy.Cli D54): read
    /// without a lock, and released only once the identity check passed.
    fn disclosed(&self, machine: &mut M) -> PrivacyHistory {
        let history = match journal::read(&self.history) {
            Ok(history) => history,
            Err(fault) => {
                return PrivacyHistory::Unreadable {
                    error: fault.into_ax("read local privacy history"),
                };
            }
        };
        // An empty history has no owner, so no identity is sampled for it.
        let released = history.holdings(|recorded| match recorded {
            None => Ok(()),
            Some(owner) => {
                let identity = machine.identity()?;
                machine.owner(Some(owner), &identity).map(drop)
            }
        });
        match released {
            Ok(holdings) => super::answer::disclosed(&holdings),
            Err(error) => PrivacyHistory::Withheld { error },
        }
    }

    /// The results table. A holder that stopped halfway can only have
    /// left a whole table behind, because every change to it is one push,
    /// one pop or one assignment.
    fn kept(&self) -> MutexGuard<'_, Kept> {
        self.kept.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Kept {
    fn record(&mut self, outcome: PrivacyOutcome) {
        self.outcomes.push_back(outcome);
        while self.outcomes.len() > KEPT {
            self.outcomes.pop_front();
        }
    }

    fn settle(&mut self, idem: IdemKey, result: PrivacyResult) {
        if let Some(outcome) = self
            .outcomes
            .iter_mut()
            .find(|outcome| outcome.idem == idem)
        {
            outcome.result = result;
        }
    }

    /// Drops an accepted operation that never started.
    fn forget(&mut self, idem: IdemKey) {
        self.accepted.remove(&idem);
        self.outcomes.retain(|outcome| outcome.idem != idem);
    }
}

fn command(action: &PrivacyAction) -> Result<Command, AxError> {
    Ok(match action {
        PrivacyAction::Apply { control, expected } => Command::Apply {
            control: *control,
            expected: Snapshot::try_from(expected)?,
        },
        PrivacyAction::Restore { control, expected } => Command::Restore {
            control: *control,
            expected: Snapshot::try_from(expected)?,
        },
        PrivacyAction::Reconcile { expected } => Command::Reconcile {
            expected: Snapshot::try_from(expected)?,
        },
    })
}

fn result(done: Done) -> PrivacyResult {
    match done {
        Done::Applied { operation } => PrivacyResult::Applied {
            operation: operation.get(),
        },
        Done::Restored { operation } => PrivacyResult::Restored {
            operation: operation.get(),
        },
        Done::AlreadyWritten => PrivacyResult::AlreadyWritten,
        Done::Reconciled {
            operation,
            settlement,
        } => PrivacyResult::Reconciled {
            operation: operation.get(),
            settlement,
        },
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
    use std::collections::BTreeMap;

    use kernel::{RunId, SecretRef, Seq, TimeMs};
    use wire::{PrivacyControl, PrivacyCurrent, PrivacyFaultCode, PrivacyValue, PrivacyWindows};

    use super::super::fault::{ReadFault, WriteFault};
    use super::super::state::fixtures::{dword, owner};
    use super::super::target::{RawValue, Reading};
    use super::*;

    /// What the test host holds, shared by every machine a service makes.
    #[derive(Default)]
    struct World {
        values: BTreeMap<PrivacyControl, Snapshot>,
        foreign: bool,
        not_windows: bool,
        identities: usize,
        reads: usize,
        writes: usize,
        /// Runs once, at the next identity sample, outside the world's lock.
        on_identity: Option<Box<dyn FnOnce() + Send>>,
    }

    #[derive(Clone, Default)]
    struct TestMachine(Arc<Mutex<World>>);

    impl TestMachine {
        fn world(&self) -> MutexGuard<'_, World> {
            self.0.lock().unwrap()
        }
    }

    impl accounting::Clock for TestMachine {
        fn now(&self) -> Result<TimeMs, AxError> {
            Ok(TimeMs::new(1_000))
        }
    }

    impl Host for TestMachine {
        type Identity = ();

        fn identity(&mut self) -> Result<(), AxError> {
            let hook = {
                let mut world = self.world();
                world.identities += 1;
                world.on_identity.take()
            };
            if let Some(hook) = hook {
                hook();
            }
            Ok(())
        }

        fn owner(&mut self, _: Option<&SecretRef>, (): &()) -> Result<SecretRef, AxError> {
            if self.world().foreign {
                Err(AxError::failure(
                    AxCode::ConfigInvalid,
                    "verify privacy owner",
                    "another identity",
                )
                .with_recovery("fixture"))
            } else {
                Ok(owner())
            }
        }

        fn read(&mut self, control: PrivacyControl) -> Result<Reading, ReadFault> {
            let mut world = self.world();
            world.reads += 1;
            Ok(Reading {
                value: world
                    .values
                    .get(&control)
                    .cloned()
                    .unwrap_or(Snapshot::Registry(RawValue::Absent)),
                key_existed: true,
            })
        }

        fn write(&mut self, control: PrivacyControl, value: &Snapshot) -> Result<(), WriteFault> {
            let mut world = self.world();
            world.writes += 1;
            world.values.insert(control, value.clone());
            Ok(())
        }
    }

    impl Machine for TestMachine {
        fn facts(&mut self) -> PrivacyHost {
            if self.world().not_windows {
                PrivacyHost::NotWindows
            } else {
                PrivacyHost::Windows(PrivacyWindows {
                    edition_id: "Professional".to_owned(),
                    edition: Some(wire::PrivacyEdition::Pro),
                    build: "26100.1".to_owned(),
                    display_version: Some("24H2".to_owned()),
                })
            }
        }
    }

    type Fixture = (tempfile::TempDir, TestMachine, Arc<Service<TestMachine>>);

    fn served() -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let machine = TestMachine::default();
        let made = machine.clone();
        let path = accounting::home::Home::at(dir.path()).privacy_history();
        let service = Arc::new(Service::new(move || made.clone(), path));
        (dir, machine, service)
    }

    fn idem(n: u64) -> IdemKey {
        IdemKey::derive(&RunId::CITY, Seq::FIRST, &n.to_le_bytes())
    }

    const CONTROL: PrivacyControl = PrivacyControl::StartLaunchTracking;

    fn apply(expected: PrivacyValue) -> PrivacyAction {
        PrivacyAction::Apply {
            control: CONTROL,
            expected,
        }
    }

    fn run(service: &Service<TestMachine>, action: PrivacyAction, idem: IdemKey) {
        let operation = service.accept(action, idem).unwrap().unwrap();
        service.perform(operation);
    }

    fn result(service: &Service<TestMachine>, idem: IdemKey) -> PrivacyResult {
        let answer = service.answer();
        let outcome = answer.outcomes.iter().find(|outcome| outcome.idem == idem);
        outcome.unwrap().result.clone()
    }

    /// The account the history belongs to sees the value this app found;
    /// any other account is answered without a single recorded value, and
    /// the current values are still read for it.
    #[test]
    fn only_the_owning_account_is_shown_a_recorded_value() {
        let (_dir, machine, service) = served();
        machine.world().values.insert(CONTROL, dword(1));
        run(&service, apply(PrivacyValue::Dword { number: 1 }), idem(1));
        assert_eq!(
            result(&service, idem(1)),
            PrivacyResult::Applied { operation: 1 }
        );

        let mine = service.answer();
        let PrivacyHistory::Disclosed { owned, unresolved } = &mine.history else {
            panic!("the owner is shown the history: {:?}", mine.history);
        };
        assert_eq!(unresolved, &None);
        let found: Vec<_> = owned
            .iter()
            .map(|intent| (intent.control, &intent.original))
            .collect();
        assert_eq!(found, [(CONTROL, &PrivacyValue::Dword { number: 1 })]);

        machine.world().foreign = true;
        let theirs = service.answer();
        assert!(matches!(theirs.history, PrivacyHistory::Withheld { .. }));
        let entry = theirs
            .controls
            .iter()
            .find(|entry| entry.control == CONTROL)
            .unwrap();
        assert_eq!(
            entry.current,
            PrivacyCurrent::Read {
                value: PrivacyValue::Dword { number: 0 }
            }
        );
    }

    /// An empty history has no owner: no identity is sampled to answer.
    #[test]
    fn an_empty_history_samples_no_identity() {
        let (_dir, machine, service) = served();
        let answer = service.answer();
        assert_eq!(
            answer.history,
            PrivacyHistory::Disclosed {
                owned: Vec::new(),
                unresolved: None
            }
        );
        assert_eq!(answer.controls.len(), PrivacyControl::ALL.len());
        assert_eq!(machine.world().identities, 0);
    }

    /// A host that is not Windows is not read, and every control says so.
    #[test]
    fn a_host_that_is_not_windows_is_not_read() {
        let (_dir, machine, service) = served();
        machine.world().not_windows = true;
        let answer = service.answer();
        assert!(
            answer
                .controls
                .iter()
                .all(|entry| entry.current == PrivacyCurrent::NotRead)
        );
        assert_eq!(machine.world().reads, 0);
    }

    /// A value spelled otherwise than the host spells it is refused before
    /// anything is kept, and the same idem may then be sent right.
    #[test]
    fn a_value_not_spelled_by_the_host_keeps_nothing() {
        let (_dir, machine, service) = served();
        let wrong = PrivacyValue::Raw {
            kind: 4,
            hex: "01000000".to_owned(),
        };
        let refused = service.accept(apply(wrong), idem(1)).map(drop).unwrap_err();
        assert_eq!(*refused.code(), AxCode::InvalidArgs);
        assert!(service.answer().outcomes.is_empty());
        machine.world().values.insert(CONTROL, dword(1));
        let right = service.accept(apply(PrivacyValue::Dword { number: 1 }), idem(1));
        assert!(right.unwrap().is_some());
    }

    /// One idem is carried out once: a replay while its result is kept
    /// does nothing, and a replay after its result gave way is refused
    /// rather than run against a host that has moved on.
    #[test]
    fn an_idem_is_carried_out_once() {
        let (_dir, machine, service) = served();
        let check = || PrivacyAction::Reconcile {
            expected: PrivacyValue::Absent,
        };
        for n in 0..=u64::try_from(KEPT).unwrap() {
            run(&service, check(), idem(n));
        }
        assert!(service.accept(check(), idem(1)).unwrap().is_none());
        let first = service.accept(check(), idem(0)).map(drop).unwrap_err();
        assert_eq!(*first.code(), AxCode::InvalidArgs);
        let answer = service.answer();
        assert_eq!(answer.outcomes.len(), KEPT);
        assert!(answer.outcomes.iter().all(|outcome| matches!(
            outcome.result,
            PrivacyResult::Refused {
                code: PrivacyFaultCode::NothingUnresolved,
                ..
            }
        )));
        assert_eq!(machine.world().writes, 0);
    }

    /// An operation that concludes while an answer is being read is never
    /// reported concluded by an answer whose history lacks its conclusion
    /// (Privacy.Service D71): the page stops asking on that answer and draws
    /// its ownership and buttons from it.
    #[test]
    fn an_answer_reports_no_conclusion_its_history_lacks() {
        let (_dir, machine, service) = served();
        let other = PrivacyControl::FeedbackNotifications;
        machine.world().values.insert(CONTROL, dword(1));
        let first = PrivacyAction::Apply {
            control: other,
            expected: PrivacyValue::Absent,
        };
        run(&service, first, idem(1));
        assert_eq!(
            result(&service, idem(1)),
            PrivacyResult::Applied { operation: 1 }
        );
        let operation = service
            .accept(apply(PrivacyValue::Dword { number: 1 }), idem(2))
            .unwrap()
            .unwrap();
        let performer = Arc::clone(&service);
        machine.world().on_identity = Some(Box::new(move || performer.perform(operation)));

        let answer = service.answer();
        let result = &answer
            .outcomes
            .iter()
            .find(|outcome| outcome.idem == idem(2))
            .unwrap()
            .result;
        let PrivacyHistory::Disclosed { owned, .. } = &answer.history else {
            panic!("the owner is shown the history: {:?}", answer.history);
        };
        let owns = owned.iter().any(|intent| intent.control == CONTROL);
        assert!(
            owns || *result == PrivacyResult::Running,
            "the answer reports {result:?} while its history does not own {CONTROL:?}"
        );
        assert_eq!(machine.world().values[&CONTROL], dword(0));
    }

    /// An accepted operation reads as running until it is performed, and
    /// then as what the coordinator concluded.
    #[test]
    fn an_accepted_operation_runs_until_performed() {
        let (_dir, machine, service) = served();
        machine.world().values.insert(CONTROL, dword(1));
        let operation = service
            .accept(apply(PrivacyValue::Dword { number: 1 }), idem(7))
            .unwrap()
            .unwrap();
        assert_eq!(result(&service, idem(7)), PrivacyResult::Running);
        service.perform(operation);
        assert_eq!(
            result(&service, idem(7)),
            PrivacyResult::Applied { operation: 1 }
        );
        assert_eq!(machine.world().values[&CONTROL], dword(0));
    }
}
