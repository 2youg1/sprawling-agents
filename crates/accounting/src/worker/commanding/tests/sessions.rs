// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A session's name and a room's run policy, through the worker's own
//! door (`crates/wire/spec/Answer/Sessions.lean` D27).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use kernel::event::Who;
use kernel::event::record::{RunPolicyChanged, SessionNamed};
use kernel::model::{Mode, RunPolicy};

use crate::worker::*;

fn room() -> Address {
    Address::parse("hall/mayor").unwrap()
}

fn key(name: &[u8]) -> kernel::IdemKey {
    kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, name)
}

fn city() -> (tempfile::TempDir, RunWorker) {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    let worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();
    (dir, worker)
}

/// Every line of one kind, as the room it was written at and its payload.
fn lines<T: serde::de::DeserializeOwned>(root: &Path, kind: &str) -> Vec<(String, T)> {
    let ledger = kernel::layout::CityLayout::new(root).ledger();
    runtime::replay::verify_ledger_dir(&ledger)
        .unwrap()
        .raw_lines()
        .iter()
        .map(|line| serde_json::from_slice::<serde_json::Value>(line).unwrap())
        .filter(|value| value["kind"] == kind)
        .map(|value| {
            (
                value["addr"].as_str().unwrap().to_owned(),
                serde_json::from_value(value["data"].clone()).unwrap(),
            )
        })
        .collect()
}

fn name(began: kernel::Seq, name: &str, idem: &[u8]) -> wire::Command {
    wire::Command::NameSession(wire::SessionNaming {
        room: room(),
        began,
        name: name.to_owned(),
        idem: key(idem),
    })
}

/// A name lands as one `session_named` line at the room, and a name that
/// cannot sit on one line is refused before anything is written.
#[test]
fn naming_a_session_records_the_name_at_its_room() {
    let (dir, mut worker) = city();
    let began = kernel::Seq::FIRST;
    worker.handle(name(began, "kiln plans", b"named")).unwrap();
    let refused = worker
        .handle(name(began, "two\nlines", b"refused"))
        .unwrap_err();
    assert_eq!(refused.code().as_str(), "E_INVALID_ARGS");
    assert_eq!(
        lines::<SessionNamed>(dir.path(), "session_named"),
        vec![(
            "hall/mayor".to_owned(),
            SessionNamed {
                began,
                name: "kiln plans".to_owned(),
            }
        )]
    );
}

/// A new run policy lands as one `run_policy_changed` line at the room,
/// written by the person.
#[test]
fn changing_a_run_policy_records_the_policy_the_person_chose() {
    let (dir, mut worker) = city();
    let policy = RunPolicy::of(Mode::Chat);
    worker
        .handle(wire::Command::ChangeRunPolicy(wire::PolicyChange {
            room: room(),
            policy,
            idem: key(b"policy"),
        }))
        .unwrap();
    assert_eq!(
        lines::<RunPolicyChanged>(dir.path(), "run_policy_changed"),
        vec![(
            "hall/mayor".to_owned(),
            RunPolicyChanged {
                policy,
                by: Who::Person,
            }
        )]
    );
}

/// A new run policy for a room a run is working in reaches that run: it
/// waits in the slot the run's safe points read, and only the last
/// change waits (`crates/runtime/spec/PolicyTake.lean` §8-62).
#[test]
fn changing_the_policy_of_a_worked_room_reaches_the_run_working_there() {
    let (_dir, mut worker) = city();
    let running = RunId::from_bytes([9u8; 16]);
    let lent = worker.collaborating.rooms.lend(&room(), running);
    let chat = RunPolicy::of(Mode::Chat);
    let tighter = RunPolicy {
        write: kernel::WriteLimit::Create,
        ..RunPolicy::of(Mode::Work)
    };
    for (policy, idem) in [(chat, b"first".as_slice()), (tighter, b"second".as_slice())] {
        worker
            .handle(wire::Command::ChangeRunPolicy(wire::PolicyChange {
                room: room(),
                policy,
                idem: key(idem),
            }))
            .unwrap();
    }
    assert_eq!(
        (lent.policy.take(), lent.policy.take()),
        (Some(tighter), None)
    );
}
