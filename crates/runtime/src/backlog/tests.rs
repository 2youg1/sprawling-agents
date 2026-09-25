// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the backlog is held to from inside: one scratch directory name
//! per member of one backlog of one process, and a run's end that
//! reaches every command the run left running.

use super::Backlog;

/// Two backlogs opened separately must not name one directory.
///
/// Judged on the name rather than by racing two commands, because
/// the defect is not a race: both backlogs mint id 1 and the old
/// name was built from the id alone, so the collision is certain and
/// only the damage was timing-dependent. A city ran a command that
/// exited zero and produced nothing, because the other backlog's
/// `collect` had deleted the directory this one was writing into.
#[test]
fn two_backlogs_of_one_process_name_different_directories() {
    let mine = Backlog::new();
    let theirs = Backlog::new();
    let first = mine.mint().unwrap();
    assert_eq!(
        first,
        theirs.mint().unwrap(),
        "the ids collide by design; the directories are what must not"
    );
    assert_ne!(mine.scratch.dir(first), theirs.scratch.dir(first));
}

/// A clone is another handle onto one backlog, so it keeps naming
/// that backlog's directories rather than opening its own.
#[test]
fn a_clone_shares_the_directories_of_the_backlog_it_came_from() {
    let mine = Backlog::new();
    let id = mine.mint().unwrap();
    assert_eq!(mine.clone().scratch.dir(id), mine.scratch.dir(id));
}

/// One backlog's members never share a directory either.
#[test]
fn two_members_of_one_backlog_name_different_directories() {
    let mine = Backlog::new();
    assert_ne!(
        mine.scratch.dir(mine.mint().unwrap()),
        mine.scratch.dir(mine.mint().unwrap())
    );
}

/// A command that outlives the window by `seconds`, started for `owner`.
fn a_background_command(backlog: &Backlog, owner: kernel::RunId, seconds: u8) {
    let mut slow = if cfg!(windows) {
        let mut command = std::process::Command::new("ping");
        command.args(["-n", &seconds.to_string(), "127.0.0.1"]);
        command
    } else {
        let mut command = std::process::Command::new("sleep");
        command.arg(seconds.to_string());
        command
    };
    slow.current_dir(std::env::temp_dir());
    let addr = kernel::Address::parse("vault/room1").unwrap();
    let started = backlog.run(owner, &addr, "slow".to_owned(), slow).unwrap();
    assert!(matches!(started, crate::Started::Backgrounded { .. }));
}

/// A run's end reaches the table even after another thread died holding
/// it. `release` is called from `ExecTool`'s drop, which has nobody to
/// hand a failure to, so a release that could fail would leave the
/// ended run's commands running for a run that no longer exists.
#[test]
fn a_release_reaches_a_table_a_dead_thread_left_locked() {
    let backlog = Backlog::with_window(crate::PollBudget::new(1, 1));
    let ended = kernel::RunId::from_bytes([1; 16]);
    a_background_command(&backlog, ended, 2);
    let table = backlog.table.clone();
    let died = std::thread::spawn(move || {
        let _held = table.lock().unwrap();
        panic!("the thread dies holding the table");
    })
    .join();
    assert!(died.is_err() && backlog.table.is_poisoned());
    assert_eq!(backlog.release(ended).ok(), Some(1));
}

/// A run's end terminates what it left running: nothing can read its
/// outcome any more, and a sandboxed command's copied tree is deleted
/// in the same drop that releases it.
#[test]
fn a_release_terminates_what_the_ended_run_left_running() {
    let backlog = Backlog::with_window(crate::PollBudget::new(1, 1));
    let (ended, other) = (
        kernel::RunId::from_bytes([1; 16]),
        kernel::RunId::from_bytes([2; 16]),
    );
    a_background_command(&backlog, ended, 60);
    backlog.release(ended).unwrap();
    let addr = kernel::Address::parse("vault/room1").unwrap();
    for _ in 0..250 {
        assert!(backlog.harvest(other).unwrap().is_empty());
        if backlog.standing(&addr).unwrap().is_empty() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    panic!("a released command is still running five seconds later");
}
