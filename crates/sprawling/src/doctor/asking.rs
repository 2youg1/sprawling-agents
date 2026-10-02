// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One program asked one question, its exit code and what it wrote to
//! stdout read back: how `doctor::github` asks `gh` for a login and
//! `doctor::scanning` asks `fsutil` and PowerShell about the city's disk.
//!
//! **It never waits on a person, and its wait is counted.** stdin is
//! empty and stderr is dropped; the wait is a number of knocks at
//! `TICK`, as `doctor::running`'s is, so no clock is read here; and a
//! program still running when the knocks run out is stopped through
//! `running::stop`, the one place this binary decides what stopping a
//! child means.
//!
//! stdout is read to its end on a thread of its own while the knocks go
//! on: a Windows pipe holds a few kilobytes, and a program that fills
//! it with nobody reading would sit on its write until the knocks ran
//! out, answering nothing however fast it was.

use std::io::Read as _;
use std::process::{ChildStdout, Command, Stdio};
use std::sync::mpsc::Receiver;
use std::time::Duration;

/// How long this city waits between two knocks.
pub(super) const TICK: Duration = Duration::from_millis(50);

/// How one call ended.
pub(super) enum Ended {
    /// It exited; `code` is absent when a signal ended it. `stdout` is
    /// what it wrote, a byte that is not UTF-8 replaced rather than the
    /// whole answer dropped.
    Exited { code: Option<i32>, stdout: String },
    /// It would not start.
    Unstarted,
    /// It did not answer in time, or could not be watched, and was
    /// stopped; `stopping` is what went wrong while stopping it.
    Unanswered { stopping: Option<String> },
}

/// Starts `command` with stdin empty, stdout read and stderr dropped,
/// and waits for it, `knocks` knocks at most.
pub(super) fn ask(command: &mut Command, knocks: u32) -> Ended {
    let started = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();
    let Ok(mut child) = started else {
        return Ended::Unstarted;
    };
    let printed = child.stdout.take().map(drain);
    let mut left = knocks;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                return Ended::Exited {
                    code: status.code(),
                    stdout: printed.map_or_else(String::new, |printed| {
                        collected(&printed, TICK.saturating_mul(left.max(1)))
                    }),
                };
            }
            Ok(None) => {}
            Err(_unwatchable) => {
                return Ended::Unanswered {
                    stopping: super::running::stop(&mut child),
                };
            }
        }
        let Some(fewer) = left.checked_sub(1) else {
            return Ended::Unanswered {
                stopping: super::running::stop(&mut child),
            };
        };
        left = fewer;
        std::thread::sleep(TICK);
    }
}

/// Reads `stdout` to its end on a thread of its own. A pipe that will
/// not read, or a thread that will not start, yields nothing, which
/// every caller reads as an answer it cannot use.
fn drain(mut stdout: ChildStdout) -> Receiver<Vec<u8>> {
    let (sender, printed) = std::sync::mpsc::channel();
    let reading = std::thread::Builder::new()
        .name("doctor-asking".to_owned())
        .spawn(move || {
            let mut bytes = Vec::new();
            let read = match stdout.read_to_end(&mut bytes) {
                Ok(_) => bytes,
                Err(_unreadable) => Vec::new(),
            };
            // Nobody waiting any more is the one way this send fails, and
            // then there is nobody to tell.
            drop(sender.send(read));
        });
    // A reader that never started drops its sender with the closure, so
    // the wait below ends at once with nothing.
    drop(reading);
    printed
}

/// What the reader collected, waiting at most `patience` for a pipe a
/// grandchild may still hold open after the program itself exited.
fn collected(printed: &Receiver<Vec<u8>>, patience: Duration) -> String {
    match printed.recv_timeout(patience) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(_late_or_gone) => String::new(),
    }
}
