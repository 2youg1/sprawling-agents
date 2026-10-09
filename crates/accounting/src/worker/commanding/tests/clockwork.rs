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

use crate::worker::CommandDesk;
use crate::worker::desk::DeskWait;
use crate::worker::fixture::*;
use crate::worker::*;

#[test]
fn a_scheduled_job_starts_by_itself_and_only_once_per_firing() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
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

    worker.land_the_rest().unwrap();

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
    let desk = CommandDesk::default();
    let asked = || wire::Command::Dispatch {
        addr: Address::parse("lab/room1").unwrap(),
        task: "read the plan".to_owned(),
        goal: "one answer".to_owned(),
        policy: kernel::RunPolicy::of(kernel::Mode::Work),
        idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"lab/room1|read the plan"),
        session: None,
        effort: None,
        model: None,
    };

    desk.post(asked(), wire::Reply::nowhere());
    desk.post(asked(), wire::Reply::nowhere());
    let carrying = desk.next(|_| false);
    assert!(
        matches!(carrying, DeskWait::Command(..)),
        "the first ask is taken off the desk"
    );
    assert!(
        matches!(desk.next(|_| false), DeskWait::Idle),
        "a second frame of the same ask is a second bill, not a second piece of work"
    );

    // A repeat that arrives while the work is going is the same ask
    // once more: the run it wants is already running.
    desk.post(asked(), wire::Reply::nowhere());
    assert!(
        matches!(desk.next(|_| false), DeskWait::Idle),
        "the ask is still being carried out; a repeat adds nothing"
    );

    // ...and once it is over, asking again is asking for a second
    // run, which is a thing a person is allowed to want.
    drop(carrying);
    desk.post(asked(), wire::Reply::nowhere());
    assert!(
        matches!(desk.next(|_| false), DeskWait::Command(..)),
        "the same work asked for again after it finished is work"
    );
}

/// Work already accepted finishes first: a close that dropped a
/// queued command would make "stopped" and "lost" the same thing in
/// the record.
#[test]
fn a_close_lands_between_commands_and_never_inside_one() {
    let desk = CommandDesk::default();
    desk.post(
        wire::Command::CreateBuilding {
            addr: Address::parse("lab").unwrap(),
            template: wire::TemplateName::parse("minimal").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"create"),
        },
        wire::Reply::nowhere(),
    );
    desk.close(Closing::Chosen {
        by: crate::worker::ClosedBy::Console,
        mode: wire::CloseMode::Drain,
    });

    assert!(
        matches!(desk.next(|_| false), DeskWait::Command(..)),
        "the queued command was dropped by the close"
    );
    assert!(matches!(
        desk.next(|_| false),
        DeskWait::Close(Closing::Chosen { .. })
    ));
}

/// A serve that failed closes the city as `Broken`, not through the door
/// `/quit` uses: a handoff saying the person closed it would record a
/// choice nobody made and a failure the next session never heard of.
#[test]
fn a_city_that_serving_brought_down_does_not_say_the_person_closed_it() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();
    let desk = CommandDesk::default();
    desk.close(Closing::Broken {
        cause: "the listener is gone".to_owned(),
    });
    crate::worker::attend::attend(&mut worker, &desk);

    let verified = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
    let last = verified
        .raw_lines()
        .last()
        .map(|line| String::from_utf8_lossy(line).into_owned())
        .expect("the ledger has a last line");
    assert!(last.contains("handoff_written"), "{last}");
    assert!(
        !last.contains("the city was closed"),
        "a failure is recorded as a choice: {last}"
    );
    assert!(
        last.contains("the listener is gone"),
        "the next session is not told what failed: {last}"
    );
}

/// One job in a window must not take the rest of that window with it:
/// a tick that closed the window first and returned on the first refusal
/// would leave the other jobs due that minute unrun, with nothing
/// recording that they had not run. Every due job is attempted, and a
/// refusal is written to the log instead of to the other jobs.
#[test]
fn a_scheduled_job_that_cannot_start_does_not_take_the_others_with_it() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    std::fs::write(
        city::schedule_path(dir.path()),
        "[[job]]\nname = \"nowhere\"\naddr = \"ghost/room1\"\n\
         task = \"sweep a building that is not there\"\ngoal = \"nothing\"\n\
         every = \"15m\"\n\
         \n[[job]]\nname = \"sweep\"\naddr = \"lab/room1\"\n\
         task = \"sweep the roadmap\"\ngoal = \"every row has a status\"\n\
         every = \"15m\"\n",
    )
    .unwrap();
    let (base_url, _provider) = fake_openai(&["m-local"], vec![completion("done", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();

    let minute = 60_000;
    worker.last_tick = kernel::TimeMs::new(14 * minute);
    // Both are taken into a lane, because a lane is where a dispatch
    // is judged now: the schedule's job is a request, and whether the
    // building behind it exists is the drive's answer rather than the
    // tick's. What the tick promises is that it walked the whole
    // window.
    assert_eq!(
        worker.tick(kernel::TimeMs::new(15 * minute)).unwrap(),
        2,
        "the job behind the one that cannot start is still owed its run"
    );
    worker.land_the_rest().unwrap();

    let verified = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
    // The two lanes drive at once, so which opening lands first is a
    // race; the promise is that each did once, not their order.
    let mut started: Vec<String> = verified
        .raw_lines()
        .iter()
        .filter_map(|line| serde_json::from_slice::<serde_json::Value>(line).ok())
        .filter(|value| value["kind"] == "run_started")
        .filter_map(|value| value["addr"].as_str().map(str::to_owned))
        .collect();
    started.sort();
    assert_eq!(
        started,
        vec!["ghost/room1".to_owned(), "lab/room1".to_owned()],
        "every job due in the window reached a lane; what the mistyped one          cannot do is stop the one behind it"
    );
}
