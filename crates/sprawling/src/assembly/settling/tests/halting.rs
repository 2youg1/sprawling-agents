// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use crate::assembly::fixture::*;
use crate::assembly::*;

/// A run a resident handed down is a member of the backlog, so a halt
/// on its scope reaches it (runtime-SPEC 8-28-2).
///
/// Before this, the table held background commands only: a halted
/// building's delegated run went on to its freeze as if nobody had
/// said stop. The interrupt source stands in for the accounting
/// thread here - it halts the building the moment a run member is
/// standing in it, which is the moment the child asks at its first
/// safe point - and the child must freeze cancelled without spending
/// a model call.
#[test]
fn a_halt_on_the_building_stops_the_run_a_resident_handed_down() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    let (base_url, provider) = fake_openai(
        &["m-local"],
        vec![
            completion_with(
                "handing it down",
                "delegate",
                "tu_1",
                serde_json::json!({
                    "room": "lab/helper",
                    "task": "measure the thing",
                    "goal": "a number, then stop",
                }),
            ),
            completion("waiting on a person", None),
            completion_with(
                "handing it down",
                "delegate",
                "tu_2",
                serde_json::json!({
                    "room": "lab/helper",
                    "task": "measure the thing",
                    "goal": "a number, then stop",
                }),
            ),
            completion("done", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(channels::Command::CreateBuilding {
            addr: Address::parse("lab").unwrap(),
            template: channels::TemplateName::parse("minimal").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"create"),
        })
        .unwrap();
    let lab = Address::parse("lab").unwrap();
    let table = worker.backlog.clone();
    let halted = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counted = std::sync::Arc::clone(&halted);
    worker.attach_interrupts(std::sync::Arc::new(move |_run| {
        let standing = table.standing(&lab).unwrap();
        if standing
            .iter()
            .any(|member| member.kind == runtime::BacklogKind::Run)
        {
            let reached = table.halt(Some(&lab)).unwrap();
            counted.fetch_add(reached, std::sync::atomic::Ordering::SeqCst);
        }
        Interrupt::None
    }));
    worker
        .handle(channels::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "get it measured".to_owned(),
            goal: "the number is written down, then stop".to_owned(),
            mode: channels::ModeTag::parse("plan").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
        })
        .unwrap();
    let calls_before_the_child = provider.bodies().len();
    allow_the_one_pending_item(&mut worker);

    assert!(
        halted.load(std::sync::atomic::Ordering::SeqCst) >= 1,
        "the halt reached no run member: the delegated run never entered the backlog"
    );
    let verified = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
    let helper_frozen = verified
        .raw_lines()
        .iter()
        .map(|line| String::from_utf8_lossy(line).into_owned())
        .find(|line| line.contains("run_frozen") && line.contains("lab/helper"))
        .expect("the delegated run froze");
    assert!(
        helper_frozen.contains("cancelled"),
        "the delegated run went on after its building was halted: {helper_frozen}"
    );
    // The parent carried on for one more call after approval; the
    // child, stopped at its first safe point, made none of its own.
    let child_calls = provider
        .bodies()
        .iter()
        .skip(calls_before_the_child)
        .filter(|body| body.contains("measure the thing") && !body.contains("get it measured"))
        .count();
    assert_eq!(child_calls, 0, "a cancelled child still called the model");
    assert!(
        worker
            .backlog
            .standing(&Address::parse("lab").unwrap())
            .unwrap()
            .is_empty(),
        "a frozen run is still standing in the backlog"
    );
}
