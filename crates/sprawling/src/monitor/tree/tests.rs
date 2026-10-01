// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A child process is read while it runs and is gone once it exits.

#![allow(clippy::unwrap_used, reason = "test code")]

use std::process::{Child, Command, Stdio};
use std::time::Duration;

use super::Tree;
use crate::monitor::counters::Reads;

/// A child that waits about half a minute and does nothing else.
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

    let running = tree.read(Duration::ZERO);
    child.kill().unwrap();
    child.wait().unwrap();
    drop(child);
    let gone = tree.read(Duration::from_millis(250));

    let first = running.unwrap();
    assert_eq!(
        (
            first.processes >= 1,
            first.working_set_bytes > 0,
            first.cpu_permille,
            gone,
            tree.reads(),
            tree.seen().map(|seen| seen.peak_working_set_bytes),
        ),
        (
            true,
            true,
            None,
            None,
            Reads {
                own: 0,
                machine: 0,
                table: 2,
            },
            Some(first.working_set_bytes),
        )
    );
}
