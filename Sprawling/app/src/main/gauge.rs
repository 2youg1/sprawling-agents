// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::ffi::OsString;
use std::io::Write;
use std::num::{NonZeroU32, NonZeroUsize};
use std::process::ExitCode;
use std::time::Duration;

use kernel::consts_policy::DEFAULT_AT;
use kernel::{AxCode, AxError};
use sprawling::audience::Audience;
use sprawling::monitor::tree::{MINIMUM_BEAT_MS, Seen, Tree};
use sprawling::serving::standing::monotonic_now;

use super::exit::Exit;
use super::grammar::Arguments;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Subject {
    City { at: String, token: Option<String> },
    Process(Watching),
    Command(Running),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Watching {
    pid: u32,
    every: Every,

    beats: Option<NonZeroU32>,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Running {
    program: OsString,
    args: Vec<OsString>,
    every: Every,
    samples: NonZeroU32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Measured {
    City,
    Process,
    Command,
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Every(Duration);

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Run {
    index: u32,

    exit: Option<i32>,
    wall: Duration,
    watched: Watched,
    child: ChildPeaks,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Watched {
    beats: u64,
    seen: Option<Seen>,

    read_cost: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ChildPeaks {
    private_bytes: Option<u64>,
    working_set_bytes: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Host {
    cores: NonZeroUsize,
    physical_bytes: u64,
}

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

fn count(given: &str) -> Result<NonZeroU32, Misread> {
    given.parse().map_err(|_| Misread::OutOfRange {
        flag: "--samples",
        given: given.to_owned(),
        range: "a whole number of 1 or more",
    })
}

impl Every {
    const LONGEST_MS: u64 = 60_000;
    const USUAL: Every = Every(Duration::from_secs(1));

    fn read(given: Option<&str>) -> Result<Every, Misread> {
        let Some(given) = given else {
            return Ok(Every::USUAL);
        };
        given
            .parse::<u64>()
            .ok()
            .filter(|ms| (MINIMUM_BEAT_MS..=Every::LONGEST_MS).contains(ms))
            .map(|ms| Every(Duration::from_millis(ms)))
            .ok_or_else(|| Misread::OutOfRange {
                flag: "--every",
                given: given.to_owned(),
                range: "a whole number of milliseconds from 250 to 60000",
            })
    }
}

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
