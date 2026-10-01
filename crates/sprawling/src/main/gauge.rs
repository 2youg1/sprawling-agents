// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! `sprawling gauge`: measure a served city, a running process and its
//! descendants, or a command run n times (sprawling-SPEC.md 8-129-4).
//!
//! The readings never enter a Ledger: they are this machine's facts at
//! one moment, written to stdout for whoever asked (8-129-1). Durations
//! come from the one monotonic sampling point, process figures from
//! `monitor::tree`, shares from `monitor::spread`; this module chooses
//! the subject, runs it, and hands each reading to `lines`.

use std::ffi::OsString;
use std::io::Write;
use std::num::{NonZeroU32, NonZeroUsize};
use std::process::ExitCode;
use std::time::Duration;

use kernel::consts_policy::DEFAULT_AT;
use kernel::{AxCode, AxError};
use sprawling::audience::Audience;
use sprawling::monitor::tree::{Seen, Tree};
use sprawling::serving::standing::monotonic_now;

use super::exit::Exit;
use super::grammar::Arguments;

/// What one `gauge` line measures.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Subject {
    City { at: String, token: Option<String> },
    Process(Watching),
    Command(Running),
}

/// A running process and its descendants, read every beat.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Watching {
    pid: u32,
    every: Every,
    /// How many beats to read; `None` reads until the root exits.
    beats: Option<NonZeroU32>,
}

/// A command run `samples` times, one after another.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Running {
    program: OsString,
    args: Vec<OsString>,
    every: Every,
    samples: NonZeroU32,
}

/// The subject a flag was given to, named in a `Misread`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Measured {
    City,
    Process,
    Command,
}

/// A `gauge` line whose words the table accepted and whose subject
/// cannot be chosen from them. Every arm exits 2.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Misread {
    TwoSubjects,
    NotForThisSubject {
        flag: &'static str,
        subject: Measured,
    },
    OutOfRange {
        flag: &'static str,
        given: String,
        range: &'static str,
    },
    NothingAfterDashes,
}

/// The time between two beats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Every(Duration);

/// What one run of the command came to.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Run {
    index: u32,
    /// The command's exit code; `None` when a signal ended it.
    exit: Option<i32>,
    wall: Duration,
    watched: Watched,
    child: ChildPeaks,
}

/// What the beat thread saw of one run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Watched {
    beats: u64,
    seen: Option<Seen>,
    /// The time spent reading the process table during this run.
    read_cost: Duration,
}

/// The peaks the platform recorded for the direct child; `None` where
/// the platform gives none or refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ChildPeaks {
    private_bytes: Option<u64>,
    working_set_bytes: Option<u64>,
}

/// The class of machine a spread was taken on, without naming it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Host {
    cores: NonZeroUsize,
    physical_bytes: u64,
}

/// Measures what the line names and writes one line per reading to
/// stdout. Exit 0 once the measurement is made, whatever the measured
/// command returned; 1 when the subject cannot be started or found; 2
/// for a line no subject can be chosen from; 4 when no city answers.
pub(super) fn verb(read: &Arguments) -> ExitCode {
    let subject = match subject(read) {
        Ok(subject) => subject,
        Err(misread) => {
            eprintln!("sprawling: {misread}");
            return Exit::Line.into();
        }
    };
    let audience = Audience::of_stdout();
    let measured = match subject {
        Subject::City { at, token } => {
            return match super::wire_client::top(&at, token.as_deref(), audience) {
                Ok(()) => Exit::Done.into(),
                Err(unheard) => {
                    super::calling::tell_unheard(&unheard, super::refusal::Form::Human).into()
                }
            };
        }
        Subject::Process(watching) => watch(&watching, audience, &mut std::io::stdout().lock()),
        Subject::Command(running) => {
            running::run(&running, audience, &mut std::io::stdout().lock())
        }
    };
    match measured {
        Ok(()) => Exit::Done.into(),
        Err(err) => super::city::report(err),
    }
}

/// The subject a line names: `--pid` a process, words after `--` a
/// command, neither the city at `--at`.
///
/// # Errors
/// A `Misread` naming the flag that does not fit and the nearest line
/// that would.
pub(super) fn subject(read: &Arguments) -> Result<Subject, Misread> {
    let given = |flags: [&'static str; 2]| flags.into_iter().find(|flag| read.has(flag));
    let refuse = |flag: Option<&'static str>, subject| match flag {
        Some(flag) => Err(Misread::NotForThisSubject { flag, subject }),
        None => Ok(()),
    };
    match (read.value("--pid"), read.after_dashes()) {
        (Some(_), Some(_)) => Err(Misread::TwoSubjects),
        (None, None) => {
            refuse(given(["--every", "--samples"]), Measured::City)?;
            Ok(Subject::City {
                at: read.value("--at").unwrap_or(DEFAULT_AT).to_owned(),
                token: read.value("--token").map(str::to_owned),
            })
        }
        (Some(pid), None) => {
            refuse(given(["--at", "--token"]), Measured::Process)?;
            Ok(Subject::Process(Watching {
                pid: pid.parse().map_err(|_| Misread::OutOfRange {
                    flag: "--pid",
                    given: pid.to_owned(),
                    range: "the id of a running process",
                })?,
                every: Every::read(read.value("--every"))?,
                beats: read.value("--samples").map(count).transpose()?,
            }))
        }
        (None, Some(words)) => {
            refuse(given(["--at", "--token"]), Measured::Command)?;
            let (program, args) = words.split_first().ok_or(Misread::NothingAfterDashes)?;
            Ok(Subject::Command(Running {
                program: program.into(),
                args: args.iter().map(OsString::from).collect(),
                every: Every::read(read.value("--every"))?,
                samples: read
                    .value("--samples")
                    .map(count)
                    .transpose()?
                    .unwrap_or(NonZeroU32::MIN),
            }))
        }
    }
}

/// A `--samples` value: a whole number of at least one.
fn count(given: &str) -> Result<NonZeroU32, Misread> {
    given.parse().map_err(|_| Misread::OutOfRange {
        flag: "--samples",
        given: given.to_owned(),
        range: "a whole number of 1 or more",
    })
}

impl Every {
    const SHORTEST_MS: u64 = 250;
    const LONGEST_MS: u64 = 60_000;
    const USUAL: Every = Every(Duration::from_secs(1));

    /// An `--every` value in milliseconds: 1 s when not given. Never
    /// shorter than 250 ms, because one beat reads the whole process
    /// table, 17-65 ms on Windows (8-129-3).
    fn read(given: Option<&str>) -> Result<Every, Misread> {
        let Some(given) = given else {
            return Ok(Every::USUAL);
        };
        given
            .parse::<u64>()
            .ok()
            .filter(|ms| (Every::SHORTEST_MS..=Every::LONGEST_MS).contains(ms))
            .map(|ms| Every(Duration::from_millis(ms)))
            .ok_or_else(|| Misread::OutOfRange {
                flag: "--every",
                given: given.to_owned(),
                range: "a whole number of milliseconds from 250 to 60000",
            })
    }
}

/// Reads the tree under `watching.pid` every beat until the root exits
/// or the beats asked for are read.
///
/// # Errors
/// `E_INVALID_ARGS` when no process has the id; a failed write to `out`.
fn watch(watching: &Watching, audience: Audience, out: &mut impl Write) -> Result<(), AxError> {
    let mut tree = Tree::open(watching.pid);
    let began = monotonic_now();
    let mut previous = began;
    let mut taken = 0_u32;
    loop {
        let now = monotonic_now();
        let Some(reading) = tree.read(now.saturating_duration_since(previous)) else {
            return match taken {
                0 => Err(no_such_process(watching.pid)),
                _ => Ok(()),
            };
        };
        previous = now;
        let at = now.saturating_duration_since(began);
        emit(out, &lines::tree_line(at, &reading, audience))?;
        taken = taken.saturating_add(1);
        if watching.beats.is_some_and(|beats| taken >= beats.get()) {
            return Ok(());
        }
        std::thread::sleep(watching.every.0);
    }
}

impl Host {
    /// This machine's cores and memory. A core count the platform will
    /// not give reads as one, the least a running process has.
    fn here() -> Host {
        Host {
            cores: std::thread::available_parallelism().unwrap_or(NonZeroUsize::MIN),
            physical_bytes: sprawling::monitor::memory::read().physical,
        }
    }
}

fn emit(out: &mut impl Write, line: &str) -> Result<(), AxError> {
    writeln!(out, "{line}")
        .and_then(|()| out.flush())
        .map_err(|err| {
            AxError::failure(
                AxCode::StorageFatal,
                "write a reading to stdout",
                err.to_string(),
            )
            .with_recovery("stdout was closed; run `sprawling gauge` again")
        })
}

fn no_such_process(pid: u32) -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "read a process tree",
        format!("no running process has the id {pid}"),
    )
    .with_recovery("give the id of a running process")
}

impl std::fmt::Display for Misread {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TwoSubjects => write!(
                f,
                "gauge: --pid and -- name two things to measure; write `{}` or `{}`",
                Measured::Process.usage(),
                Measured::Command.usage()
            ),
            Self::NotForThisSubject { flag, subject } => write!(
                f,
                "gauge: {flag} does not apply to {}; write `{}`",
                subject.what(),
                subject.usage()
            ),
            Self::OutOfRange { flag, given, range } => {
                write!(f, "gauge: {flag} takes {range}, not '{given}'")
            }
            Self::NothingAfterDashes => write!(
                f,
                "gauge: -- names no program; write `{}`",
                Measured::Command.usage()
            ),
        }
    }
}

impl Measured {
    fn what(self) -> &'static str {
        match self {
            Measured::City => "a served city, which beats once a second",
            Measured::Process => "a process",
            Measured::Command => "a command",
        }
    }

    fn usage(self) -> &'static str {
        match self {
            Measured::City => "sprawling gauge [--at <addr>] [--token <t>]",
            Measured::Process => "sprawling gauge --pid <pid> [--every <ms>] [--samples <n>]",
            Measured::Command => {
                "sprawling gauge [--every <ms>] [--samples <n>] -- <program> [arg...]"
            }
        }
    }
}

#[path = "gauge/lines.rs"]
pub(crate) mod lines;
#[path = "gauge/running.rs"]
mod running;
#[cfg(test)]
#[path = "gauge/tests.rs"]
mod tests;
