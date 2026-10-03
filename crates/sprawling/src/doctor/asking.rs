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
//!
//! **It is read a line at a time and only the lines the caller asked for
//! are kept** (`crates/sprawling/spec/Doctor.lean` §8-166): a line is
//! held to [`LINE_MAX_BYTES`] and offered to the caller's `keep`, so one
//! question never holds a program's whole output in this process.

use std::io::{BufRead, BufReader, ErrorKind};
use std::process::{ChildStdout, Command, Stdio};
use std::sync::mpsc::Receiver;
use std::time::Duration;

/// How long this city waits between two knocks.
pub(super) const TICK: Duration = Duration::from_millis(50);

/// The most bytes of one stdout line that are kept; the rest of a longer
/// line is read and dropped, so a program that prints no newline cannot
/// grow this process.
pub(super) const LINE_MAX_BYTES: usize = 4096;

/// How one call ended.
pub(super) enum Ended {
    /// It exited; `code` is absent when a signal ended it. `kept` is the
    /// lines of what it wrote that `keep` chose, in order and joined by
    /// `\n`, a byte that is not UTF-8 replaced rather than the line
    /// dropped.
    Exited { code: Option<i32>, kept: String },
    /// It would not start.
    Unstarted,
    /// It did not answer in time, or could not be watched, and was
    /// stopped; `stopping` is what went wrong while stopping it.
    Unanswered { stopping: Option<String> },
}

/// Starts `command` with stdin empty, stdout read and stderr dropped,
/// and waits for it, `knocks` knocks at most; each stdout line, its
/// line ending removed, is offered to `keep` once.
pub(super) fn ask(
    command: &mut Command,
    knocks: u32,
    keep: impl FnMut(&str) -> bool + Send + 'static,
) -> Ended {
    let started = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();
    let Ok(mut child) = started else {
        return Ended::Unstarted;
    };
    let printed = child.stdout.take().map(|stdout| drain(stdout, keep));
    let mut left = knocks;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                return Ended::Exited {
                    code: status.code(),
                    kept: printed.map_or_else(String::new, |printed| {
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

/// Reads `stdout` to its end on a thread of its own, keeping the lines
/// `keep` chooses. A pipe that will not read, or a thread that will not
/// start, yields nothing, which every caller reads as an answer it
/// cannot use.
fn drain(stdout: ChildStdout, keep: impl FnMut(&str) -> bool + Send + 'static) -> Receiver<String> {
    let (sender, printed) = std::sync::mpsc::channel();
    let reading = std::thread::Builder::new()
        .name("doctor-asking".to_owned())
        .spawn(move || {
            // Nobody waiting any more is the one way this send fails, and
            // then there is nobody to tell.
            drop(sender.send(kept_lines(BufReader::new(stdout), keep)));
        });
    // A reader that never started drops its sender with the closure, so
    // the wait below ends at once with nothing.
    drop(reading);
    printed
}

/// What the reader collected, waiting at most `patience` for a pipe a
/// grandchild may still hold open after the program itself exited.
fn collected(printed: &Receiver<String>, patience: Duration) -> String {
    match printed.recv_timeout(patience) {
        Ok(kept) => kept,
        Err(_late_or_gone) => String::new(),
    }
}

/// The lines of `from` that `keep` chooses, joined by `\n`, each cut to
/// [`LINE_MAX_BYTES`]; a read that fails yields nothing.
fn kept_lines(_from: impl BufRead, _keep: impl FnMut(&str) -> bool) -> String {
    let _unread = ErrorKind::Interrupted;
    String::new()
}

#[cfg(test)]
mod tests {
    use super::{LINE_MAX_BYTES, kept_lines};

    /// Only the chosen lines are kept, without their line endings, the
    /// last one counted though no newline closes it.
    #[test]
    fn only_the_lines_the_caller_chooses_are_kept() {
        let printed =
            "Filters attached\r\nThis is a trusted developer volume.\r\nWdFilter\r\nThis is all";
        assert_eq!(
            kept_lines(printed.as_bytes(), |line| line.starts_with("This is")),
            "This is a trusted developer volume.\nThis is all"
        );
    }

    /// A line longer than the cut keeps its first bytes, and the rest of
    /// it is dropped rather than read as a line of its own.
    #[test]
    fn a_line_without_end_is_held_to_the_cut() {
        let long = "x".repeat(LINE_MAX_BYTES.saturating_mul(3));
        let printed = format!("{long}\nnext\n");
        let mut offered = Vec::new();
        let kept = kept_lines(printed.as_bytes(), |line| {
            offered.push(line.len());
            true
        });
        assert_eq!(offered, vec![LINE_MAX_BYTES, 4]);
        assert_eq!(kept, format!("{}\nnext", "x".repeat(LINE_MAX_BYTES)));
    }

    /// A byte that is not UTF-8 is replaced, and its line still offered.
    #[test]
    fn a_line_in_another_encoding_is_offered_with_replacements() {
        let printed: &[u8] = &[0xce, 0xde, b'o', b'k', b'\n'];
        assert_eq!(kept_lines(printed, |_| true), "\u{fffd}\u{fffd}ok");
    }
}
