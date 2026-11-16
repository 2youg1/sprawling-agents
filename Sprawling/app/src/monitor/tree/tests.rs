// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A child process is read while it runs and is gone once it exits.

#![allow(clippy::unwrap_used, reason = "test code")]

use std::process::{Child, Command, Stdio};
use std::time::Duration;

use super::{Tree, TreeReading};
use crate::monitor::counters::Reads;

/// A child that waits about half a minute and does nothing else.
#[expect(clippy::disallowed_methods, reason = "test fixture (child D4)")]
fn sleeper() -> Child {
    let mut command = if cfg!(windows) {
        let mut ping = Command::new("ping");
        ping.args(["-n", "30", "127.0.0.1"]);
        ping
    } else {
        let mut sleep = Command::new("sleep");
        sleep.arg("30");
        sleep
    };
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap()
}

#[test]
fn a_child_is_read_while_it_runs_and_is_gone_once_it_exits() {
    let mut child = sleeper();
    let mut tree = Tree::open(child.id());
    let mut read = Vec::new();

    // A child just spawned may not be in the table, or not yet hold its
    // own memory, on a loaded runner; the read is asked again until it is.
    let running = read_until(&mut tree, Duration::ZERO, &mut read, |reading| {
        reading.is_some_and(|reading| reading.working_set_bytes > 0)
    })
    .flatten();
    child.kill().unwrap();
    child.wait().unwrap();
    drop(child);
    // Windows wakes the waiters of a process before it takes the process
    // out of the table a snapshot walks, so a read taken right after
    // `wait` can still find the dying root, its memory mostly released.
    let gone = read_until(
        &mut tree,
        Duration::from_millis(250),
        &mut read,
        Option::is_none,
    );

    let found = running.unwrap();
    let first_beat = read.iter().flatten().next().copied();
    assert_eq!(
        (
            found.processes >= 1,
            first_beat.map(|reading| reading.cpu_permille),
            gone,
            tree.reads(),
            tree.seen().map(|seen| seen.peak_working_set_bytes),
        ),
        (
            true,
            Some(None),
            Some(None),
            Reads {
                own: 0,
                machine: 0,
                table: u64::try_from(read.len()).unwrap(),
            },
            Some(found.working_set_bytes),
        )
    );
}

/// Reads the tree until `settled` holds of a reading, 25 ms apart and at
/// most 400 times, keeping every reading in `read`; `None` when it never
/// held.
fn read_until(
    tree: &mut Tree,
    elapsed: Duration,
    read: &mut Vec<Option<TreeReading>>,
    settled: impl Fn(&Option<TreeReading>) -> bool,
) -> Option<Option<TreeReading>> {
    std::iter::repeat_with(|| {
        let reading = tree.read(elapsed);
        read.push(reading);
        if !settled(&reading) {
            std::thread::sleep(Duration::from_millis(25));
        }
        reading
    })
    .take(400)
    .find(|reading| settled(reading))
}
