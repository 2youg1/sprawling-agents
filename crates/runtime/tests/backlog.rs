// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the backlog promises a caller: a short command still feels
//! synchronous, a command that outlives its window keeps running where
//! somebody can reach it, and a command that never ends is stopped by
//! `halt` rather than by the end of the universe.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use std::process::Command;

use kernel::{Address, RunId};
use runtime::{Backlog, Started};

const RUN: RunId = RunId::CITY;

/// A program every supported host has, which does not stop on its own
/// and starts no grandchild — so terminating it terminates the whole of
/// what was started.
fn never_ends() -> Command {
    let mut command = if cfg!(windows) {
        let mut command = Command::new("ping");
        command.args(["-n", "100000", "127.0.0.1"]);
        command
    } else {
        let mut command = Command::new("sleep");
        command.arg("100000");
        command
    };
    command.current_dir(std::env::temp_dir());
    command
}

fn finishes_now(code: i32) -> Command {
    let mut command = if cfg!(windows) {
        let mut command = Command::new("cmd");
        command.args(["/C", &format!("exit {code}")]);
        command
    } else {
        let mut command = Command::new("sh");
        command.args(["-c", &format!("exit {code}")]);
        command
    };
    command.current_dir(std::env::temp_dir());
    command
}

#[test]
fn a_command_that_finishes_inside_the_window_produces_no_handle() {
    let backlog = Backlog::new();
    let addr = Address::parse("lab/room1").unwrap();
    let started = backlog
        .run(RUN, &addr, "exit 3".to_owned(), finishes_now(3))
        .unwrap();
    match started {
        Started::Settled { exit, .. } => {
            assert_eq!(exit, runtime::Exit::Ended { code: 3 });
        }
        Started::Backgrounded { .. } => panic!("a short command must still feel synchronous"),
    }
    assert!(
        backlog.standing(&addr).unwrap().is_empty(),
        "nothing is left in the table"
    );
    assert!(backlog.harvest(RUN).unwrap().is_empty());
}

#[test]
fn halt_terminates_a_command_that_never_ends_and_the_run_returns_to_a_boundary() {
    let backlog = Backlog::new();
    let addr = Address::parse("lab/room1").unwrap();
    let stopper = backlog.clone();
    let building = Address::parse("lab").unwrap();
    // The command is waited on inside the window, which is exactly where
    // a blocking `Command::output()` could not be reached.
    let brake = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(300));
        stopper.halt(Some(&building)).unwrap()
    });
    let started = backlog
        .run(RUN, &addr, "ping forever".to_owned(), never_ends())
        .unwrap();
    let reached = brake.join().unwrap();
    assert_eq!(reached, 1, "halt reached the member");
    assert!(
        matches!(started, Started::Settled { .. }),
        "the call came back to a boundary instead of staying in a system call"
    );
    assert!(backlog.standing(&addr).unwrap().is_empty());
}

#[test]
fn a_command_that_outlives_the_window_keeps_running_where_halt_can_reach_it() {
    // A one-poll window: the command never ends, so waiting out the
    // ten-second default would only make the suite ten seconds slower.
    let backlog = Backlog::with_window(runtime::PollBudget::new(1, 1));
    let addr = Address::parse("lab/room1").unwrap();
    let started = backlog
        .run(RUN, &addr, "ping forever".to_owned(), never_ends())
        .unwrap();
    let id = match started {
        Started::Backgrounded { id, .. } => id,
        Started::Settled { .. } => panic!("a command that never ends cannot have settled"),
    };
    let standing = backlog.standing(&addr).unwrap();
    assert_eq!(standing.len(), 1);
    assert_eq!(standing[0].id, id);
    assert!(
        backlog.harvest(RUN).unwrap().is_empty(),
        "nothing is reported before it stops"
    );

    assert_eq!(backlog.halt(None).unwrap(), 1);
    // The child is stopped; the result is collected once and then the
    // table forgets it.
    let mut collected = Vec::new();
    for _ in 0..100 {
        collected = backlog.harvest(RUN).unwrap();
        if !collected.is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert_eq!(collected.len(), 1, "the stopped member is reported once");
    assert_eq!(collected[0].id, id);
    assert!(backlog.harvest(RUN).unwrap().is_empty());
    assert!(backlog.standing(&addr).unwrap().is_empty());
}

/// A run that has ended leaves what it started in the background to
/// nobody: another run's harvest lets it go once it stops, and never
/// reads its output back to that other run.
#[test]
fn a_released_command_is_let_go_without_reaching_another_run() {
    let backlog = Backlog::with_window(runtime::PollBudget::new(1, 1));
    let addr = Address::parse("vault/room1").unwrap();
    let (ended, other) = (RunId::from_bytes([1; 16]), RunId::from_bytes([2; 16]));
    let mut slow = if cfg!(windows) {
        let mut command = Command::new("ping");
        command.args(["-n", "2", "127.0.0.1"]);
        command
    } else {
        let mut command = Command::new("sleep");
        command.arg("1");
        command
    };
    slow.current_dir(std::env::temp_dir());
    let started = backlog.run(ended, &addr, "slow".to_owned(), slow).unwrap();
    assert!(matches!(started, Started::Backgrounded { .. }));
    assert_eq!(backlog.release(ended).unwrap(), 1);
    for _ in 0..500 {
        assert!(backlog.harvest(other).unwrap().is_empty());
        if backlog.standing(&addr).unwrap().is_empty() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    panic!("a released command that stopped is still in the table");
}
