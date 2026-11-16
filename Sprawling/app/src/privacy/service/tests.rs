// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;

use kernel::{RunId, SecretRef, Seq, TimeMs};
use wire::{PrivacyControl, PrivacyCurrent, PrivacyFaultCode, PrivacyValue, PrivacyWindows};

use super::super::fault::{ReadFault, WriteFault};
use super::super::state::fixtures::{dword, owner};
use super::super::target::{RawValue, Reading};
use super::*;

#[derive(Default)]
struct World {
    values: BTreeMap<PrivacyControl, Snapshot>,
    foreign: bool,
    not_windows: bool,
    identities: usize,
    reads: usize,
    writes: usize,

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
