// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::float_arithmetic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]

use crate::assembly::fixture::*;
use crate::assembly::*;
use crate::serving::CommandDesk;
use crate::serving::desk::DeskWait;

#[test]
fn a_scheduled_job_starts_by_itself_and_only_once_per_firing() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    std::fs::write(
        city::schedule_path(dir.path()),
        "[[job]]\nname = \"sweep\"\naddr = \"lab/room1\"\n\
         task = \"sweep the roadmap\"\ngoal = \"every row has a status\"\n\
         every = \"15m\"\n",
    )
    .unwrap();
    let (base_url, _provider) = fake_openai(&["m-local"], vec![completion("done", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();

    // Time arrives as a parameter, so an hour passes in two calls.
    let minute = 60_000;
    worker.last_tick = kernel::TimeMs::new(14 * minute);
    assert_eq!(worker.tick(kernel::TimeMs::new(15 * minute)).unwrap(), 1);
    assert_eq!(
        worker.tick(kernel::TimeMs::new(16 * minute)).unwrap(),
        0,
        "a firing already served does not come round again inside its period"
    );
    assert_eq!(
        worker.tick(kernel::TimeMs::new(90 * minute)).unwrap(),
        1,
        "an hour of downtime owes one run, not four"
    );

    let verified = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
    let started = verified
        .raw_lines()
        .iter()
        .filter_map(|line| serde_json::from_slice::<serde_json::Value>(line).ok())
        .filter(|value| value["kind"] == "run_started")
        .count();
    assert_eq!(started, 2, "two firings, two runs, and both in the history");
}
#[test]
fn a_repeat_of_a_command_already_underway_is_not_a_second_piece_of_work() {
    let desk = CommandDesk::new();
    let moment = std::time::Duration::from_millis(1);
    let asked = || channels::Command::Dispatch {
        addr: Address::parse("lab/room1").unwrap(),
        task: "read the plan".to_owned(),
        goal: "one answer".to_owned(),
        mode: channels::ModeTag::parse("plan").unwrap(),
        budget: kernel::BudgetCap::default(),
        idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"lab/room1|read the plan"),
        session: None,
        effort: None,
    };

    desk.post(asked(), channels::Reply::nowhere());
    desk.post(asked(), channels::Reply::nowhere());
    let carrying = desk.wait(moment);
    assert!(
        matches!(carrying, DeskWait::Command(..)),
        "the first ask is taken off the desk"
    );
    assert!(
        matches!(desk.wait(moment), DeskWait::Idle),
        "a second frame of the same ask is a second bill, not a second piece of work"
    );

    // A repeat that arrives while the work is going is the same ask
    // once more: the run it wants is already running.
    desk.post(asked(), channels::Reply::nowhere());
    assert!(
        matches!(desk.wait(moment), DeskWait::Idle),
        "the ask is still being carried out; a repeat adds nothing"
    );

    // ...and once it is over, asking again is asking for a second
    // run, which is a thing a person is allowed to want.
    drop(carrying);
    desk.post(asked(), channels::Reply::nowhere());
    assert!(
        matches!(desk.wait(moment), DeskWait::Command(..)),
        "the same work asked for again after it finished is work"
    );
}

/// Work already accepted finishes first: a close that dropped a
/// queued command would make "stopped" and "lost" the same thing in
/// the record.
#[test]
fn a_close_lands_between_commands_and_never_inside_one() {
    let desk = CommandDesk::new();
    desk.post(
        channels::Command::CreateBuilding {
            addr: Address::parse("lab").unwrap(),
            template: channels::TemplateName::parse("minimal").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"create"),
        },
        channels::Reply::nowhere(),
    );
    desk.close();

    assert!(
        matches!(
            desk.wait(std::time::Duration::from_millis(1)),
            DeskWait::Command(..)
        ),
        "the queued command was dropped by the close"
    );
    assert!(matches!(
        desk.wait(std::time::Duration::from_millis(1)),
        DeskWait::Close
    ));
}
