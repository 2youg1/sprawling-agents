// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io::{BufRead, BufReader, ErrorKind};
use std::process::{ChildStdout, Command, Stdio};
use std::sync::mpsc::Receiver;
use std::time::Duration;

pub(crate) const TICK: Duration = Duration::from_millis(50);

pub(super) const LINE_MAX_BYTES: usize = 4096;

pub(crate) enum Ended {
    Exited { code: Option<i32>, kept: String },

    Unstarted,

    Unanswered { stopping: Option<String> },
}

pub(crate) fn ask(
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

fn drain(stdout: ChildStdout, keep: impl FnMut(&str) -> bool + Send + 'static) -> Receiver<String> {
    let (sender, printed) = std::sync::mpsc::channel();
    let reading = std::thread::Builder::new()
        .name("doctor-asking".to_owned())
        .spawn(move || {
            drop(sender.send(kept_lines(BufReader::new(stdout), keep)));
        });

    drop(reading);
    printed
}

#[expect(
    clippy::manual_unwrap_or_default,
    reason = "the arm names why nothing is the answer: late or gone, every caller reads it the same"
)]
fn collected(printed: &Receiver<String>, patience: Duration) -> String {
    match printed.recv_timeout(patience) {
        Ok(kept) => kept,
        Err(_late_or_gone) => String::new(),
    }
}

fn kept_lines(mut from: impl BufRead, mut keep: impl FnMut(&str) -> bool) -> String {
    let mut kept = String::new();
    let mut line = Vec::with_capacity(LINE_MAX_BYTES);
    loop {
        let available = match from.fill_buf() {
            Ok(available) => available,
            Err(interrupted) if interrupted.kind() == ErrorKind::Interrupted => continue,
            Err(_unreadable) => return String::new(),
        };
        if available.is_empty() {
            break;
        }
        let piece = available.iter().take_while(|byte| **byte != b'\n').count();
        let room = LINE_MAX_BYTES.saturating_sub(line.len());
        line.extend(available.iter().take(piece.min(room)));
        let ended = piece < available.len();
        from.consume(if ended {
            piece.saturating_add(1)
        } else {
            piece
        });
        if ended {
            offer(&mut line, &mut keep, &mut kept);
        }
    }
    if !line.is_empty() {
        offer(&mut line, &mut keep, &mut kept);
    }
    kept
}

fn offer(line: &mut Vec<u8>, keep: &mut impl FnMut(&str) -> bool, kept: &mut String) {
    let text = String::from_utf8_lossy(line);
    let text = text.strip_suffix('\r').unwrap_or(&text);
    if keep(text) {
        if !kept.is_empty() {
            kept.push('\n');
        }
        kept.push_str(text);
    }
    line.clear();
}

#[cfg(test)]
mod tests {
    use super::{LINE_MAX_BYTES, kept_lines};

    #[test]
    fn only_the_lines_the_caller_chooses_are_kept() {
        let printed =
            "Filters attached\r\nThis is a trusted developer volume.\r\nWdFilter\r\nThis is all";
        assert_eq!(
            kept_lines(printed.as_bytes(), |line| line.starts_with("This is")),
            "This is a trusted developer volume.\nThis is all"
        );
    }

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

    #[test]
    fn a_line_in_another_encoding_is_offered_with_replacements() {
        let printed: &[u8] = &[0xce, 0xde, b'o', b'k', b'\n'];
        assert_eq!(kept_lines(printed, |_| true), "\u{fffd}\u{fffd}ok");
    }
}
