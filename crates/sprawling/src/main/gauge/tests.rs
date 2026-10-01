// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A line names one subject or is misread; every reading is one whole
//! line; n runs of a failing program still end in one spread line.

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use std::num::{NonZeroU32, NonZeroUsize};
use std::time::Duration;

use kernel::consts_policy::DEFAULT_AT;
use sprawling::audience::Audience;
use sprawling::monitor::spread::Spread;
use sprawling::monitor::tree::{Seen, TreeReading};

use super::super::grammar::{Invocation, parse};
use super::super::verbs::Verb;
use super::running::run;
use super::{
    ChildPeaks, Every, Host, Measured, Misread, Run, Running, Subject, Watched, Watching, lines,
    subject,
};

fn subject_of(line: &[&str]) -> Result<Subject, Misread> {
    let words: Vec<String> = std::iter::once("gauge")
        .chain(line.iter().copied())
        .map(str::to_owned)
        .collect();
    let Ok(Invocation::Run(Verb::Gauge, read)) = parse(&words) else {
        panic!("{words:?} did not parse as gauge");
    };
    subject(&read)
}

#[test]
fn a_line_names_a_city_a_process_or_a_command() {
    assert_eq!(
        [
            subject_of(&[]),
            subject_of(&["--pid", "7", "--every", "250", "--samples", "2"]),
            subject_of(&["--samples", "3", "--", "cargo", "--version"]),
        ],
        [
            Ok(Subject::City {
                at: DEFAULT_AT.to_owned(),
                token: None,
            }),
            Ok(Subject::Process(Watching {
                pid: 7,
                every: Every(Duration::from_millis(250)),
                beats: NonZeroU32::new(2),
            })),
            Ok(Subject::Command(Running {
                program: "cargo".into(),
                args: vec!["--version".into()],
                every: Every::USUAL,
                samples: NonZeroU32::new(3).unwrap(),
            })),
        ]
    );
}

/// Each of these exits 2 through `verb`, which turns every `Misread`
/// into `Exit::Line` before anything is measured.
#[test]
fn a_line_no_subject_can_be_chosen_from_is_misread() {
    let out_of_range = |flag, given: &str, range| {
        Err(Misread::OutOfRange {
            flag,
            given: given.to_owned(),
            range,
        })
    };
    let every = "a whole number of milliseconds from 250 to 60000";
    assert_eq!(
        [
            subject_of(&["--pid", "7", "--", "cargo"]),
            subject_of(&["--at", "127.0.0.1:1", "--pid", "7"]),
            subject_of(&["--token", "t", "--", "cargo"]),
            subject_of(&["--every", "500"]),
            subject_of(&["--samples", "3"]),
            subject_of(&["--every", "249", "--", "cargo"]),
            subject_of(&["--every", "fast", "--pid", "7"]),
            subject_of(&["--samples", "0", "--", "cargo"]),
            subject_of(&["--pid", "seven"]),
            subject_of(&["--"]),
        ],
        [
            Err(Misread::TwoSubjects),
            Err(Misread::NotForThisSubject {
                flag: "--at",
                subject: Measured::Process,
            }),
            Err(Misread::NotForThisSubject {
                flag: "--token",
                subject: Measured::Command,
            }),
            Err(Misread::NotForThisSubject {
                flag: "--every",
                subject: Measured::City,
            }),
            Err(Misread::NotForThisSubject {
                flag: "--samples",
                subject: Measured::City,
            }),
            out_of_range("--every", "249", every),
            out_of_range("--every", "fast", every),
            out_of_range("--samples", "0", "a whole number of 1 or more"),
            out_of_range("--pid", "seven", "the id of a running process"),
            Err(Misread::NothingAfterDashes),
        ]
    );
}

#[test]
fn every_reading_is_one_whole_line() {
    let tree = TreeReading {
        processes: 2,
        cpu_permille: None,
        private_bytes: 100,
        working_set_bytes: 200,
        read_bytes: 300,
        written_bytes: 400,
    };
    let run = Run {
        index: 1,
        exit: Some(3),
        wall: Duration::from_micros(2_500),
        watched: Watched {
            beats: 2,
            seen: Some(Seen {
                cpu_ms: 5,
                read_bytes: 6,
                written_bytes: 7,
                peak_private_bytes: 8,
                peak_working_set_bytes: 9,
            }),
            read_cost: Duration::from_micros(40),
        },
        child: ChildPeaks {
            private_bytes: Some(10),
            working_set_bytes: None,
        },
    };
    let ms = Duration::from_millis;
    let host = Host {
        cores: NonZeroUsize::new(4).unwrap(),
        physical_bytes: 8 << 30,
    };
    let at = Duration::from_millis(1_500);
    assert_eq!(
        [
            lines::tree_line(at, &tree, Audience::Agent),
            lines::tree_line(
                at,
                &TreeReading {
                    cpu_permille: Some(123),
                    ..tree
                },
                Audience::Person
            ),
            lines::run_line(&run, Audience::Agent),
            lines::spread_line(
                &Spread::of(ms(10), [ms(20), ms(30)]),
                1,
                host,
                Audience::Agent
            ),
        ],
        [
            r#"{"line":"tree","at_us":1500000,"processes":2,"cpu_permille":null,"private_bytes":100,"working_set_bytes":200,"read_bytes":300,"written_bytes":400}"#,
            "     1.5 s  2 process(es)  cpu 12.3%  private 100 B  working set 200 B  read 300 B  written 400 B",
            r#"{"line":"run","index":1,"exit":3,"wall_us":2500,"beats":2,"seen_cpu_ms":5,"seen_read_bytes":6,"seen_written_bytes":7,"seen_peak_private_bytes":8,"seen_peak_working_set_bytes":9,"child_peak_private_bytes":10,"child_peak_working_set_bytes":null,"read_cost_us":40}"#,
            r#"{"line":"spread","samples":3,"failed":1,"floor_us":10000,"p50_us":20000,"p95_us":30000,"p99_us":30000,"peak_us":30000,"suspicious":0,"cores":4,"physical_bytes":8589934592}"#,
        ]
        .map(str::to_owned)
    );
}

/// A program that exits 3 straight away, through this platform's shell.
fn failing(samples: u32) -> Running {
    let (program, args): (&str, &[&str]) = if cfg!(windows) {
        ("cmd", &["/C", "exit 3"])
    } else {
        ("sh", &["-c", "exit 3"])
    };
    Running {
        program: program.into(),
        args: args.iter().map(Into::into).collect(),
        every: Every::USUAL,
        samples: NonZeroU32::new(samples).unwrap(),
    }
}

/// The measured command failing is data, not a refusal: every run is a
/// line with its exit code, and the spread counts the failures.
#[test]
fn three_runs_of_a_failing_program_end_in_one_spread_line() {
    let mut out = Vec::new();
    let measured = run(&failing(3), Audience::Agent, &mut out);
    let read: Vec<(String, Option<i64>)> = String::from_utf8(out)
        .unwrap()
        .lines()
        .map(|line| {
            let value: serde_json::Value = serde_json::from_str(line).unwrap();
            let told = value.get("exit").or_else(|| value.get("failed"));
            (
                value["line"].as_str().unwrap().to_owned(),
                told.and_then(serde_json::Value::as_i64),
            )
        })
        .collect();
    let line = |kind: &str, told| (kind.to_owned(), Some(told));
    assert_eq!(
        (measured.is_ok(), read),
        (
            true,
            vec![
                line("run", 3),
                line("run", 3),
                line("run", 3),
                line("spread", 3)
            ]
        )
    );
}

#[test]
fn a_program_nobody_can_find_is_a_path_not_found() {
    let missing = Running {
        program: "no-such-program-sprawling-gauge-measures".into(),
        ..failing(1)
    };
    let refused = run(&missing, Audience::Agent, &mut Vec::new()).unwrap_err();
    assert_eq!(*refused.code(), kernel::AxCode::PathNotFound);
}
