// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A model call timed by the rounds' pairing rule, and a commit's
//! evidence read only up to the cutoff, from one fold of the views per
//! export, and a commit's nearby walk read from its span on
//! (`crates/accounting/spec/Playback/Traced.lean` §8-25).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::disallowed_methods,
    reason = "test code: the instrument samples its own clock"
)]

use kernel::{EventKind, RunId};
use serde_json::{Value, json};

use super::super::{Asked, City, Verdict, check, export};
use super::evidence::{answered, called, committed_in, versioned};
use super::{Line, confidential, parsed, person, run, write};

fn attempt(model: &str) -> Value {
    json!({"segments": [], "model": model})
}

fn reply() -> Value {
    json!({"message": {}, "calls": 0})
}

/// Run 1 asks a model four times: once before lines recorded their own
/// moment; twice where a repaired resend replaced the first attempt; and
/// once with no reply yet.
#[test]
fn a_model_call_is_timed_only_where_both_its_moments_were_measured() {
    let dir = tempfile::tempdir().unwrap();
    let r1 = run(1);
    let at = Some("lab/a");
    let written = versioned(vec![
        (
            1,
            0,
            (RunId::CITY, None, EventKind::CityInitialized, json!({})),
        ),
        (1, 1_000, (r1, at, EventKind::RunStarted, json!({}))),
        (1, 1_000, (r1, at, EventKind::ModelCalled, attempt("m-old"))),
        (1, 1_000, (r1, at, EventKind::ModelReturned, reply())),
        (2, 2_000, (r1, at, EventKind::ModelCalled, attempt("m"))),
        (2, 2_100, (r1, at, EventKind::ModelCalled, attempt("m"))),
        (2, 2_900, (r1, at, EventKind::ModelReturned, reply())),
        (2, 3_000, (r1, at, EventKind::ModelCalled, attempt("m"))),
    ]);
    write(dir.path(), &written, b"");
    let model = |name: &str| json!({"model": {"name": name}});
    let r1 = r1.to_string();
    assert_eq!(
        parsed(export(dir.path(), &person()).unwrap().bytes())["calls"],
        json!([
            {"run": r1, "callee": model("m-old"),
             "called": {"at": "2"}, "answered": {"at": "3"}, "took": "unknown"},
            {"run": r1, "callee": model("m"),
             "called": {"at": "4"}, "answered": "pending", "took": "unknown"},
            {"run": r1, "callee": model("m"),
             "called": {"at": "5"}, "answered": {"at": "6"}, "took": {"measured": "800"}},
            {"run": r1, "callee": model("m"),
             "called": {"at": "7"}, "answered": "pending", "took": "unknown"},
        ])
    );
}

/// After the export, two lines are appended and the first of them is
/// altered, so the history after the cutoff no longer verifies. The
/// bundle's evidence was read up to its cutoff, so recomputing it from
/// the damaged city gives the same bytes.
#[test]
fn a_history_broken_after_the_cutoff_recomputes_the_same() {
    let dir = tempfile::tempdir().unwrap();
    let (mut script, _) = committed_in(dir.path());
    write(dir.path(), &super::lines(script.clone()), b"");
    confidential(dir.path(), "vault");
    let bundle = export(dir.path(), &person()).unwrap();
    script.extend([
        (run(2), Some("lab/b"), EventKind::RunStarted, json!({})),
        (
            run(2),
            Some("lab/b"),
            EventKind::ToolCalled,
            called("d1", "read"),
        ),
    ]);
    let mut lines = super::lines(script);
    let altered = lines.len() - 2;
    lines[altered] = String::from_utf8(lines[altered].clone())
        .unwrap()
        .replacen("\"tester\"", "\"altered\"", 1)
        .into_bytes();
    write(dir.path(), &lines, b"");
    let recomputed = check(
        bundle.bytes(),
        &Asked {
            city: Some(City {
                root: dir.path(),
                reader: person().reader,
            }),
            ..Asked::default()
        },
    );
    assert_eq!(recomputed.source, Verdict::Passed);
}

/// Run 1 in `lab/a` commits `commits` times, after one turn and one tool
/// call each; before each turn, another run in another building makes
/// `padding` tool calls, so the history grows without adding commits.
fn committing(commits: u8, padding: usize) -> Vec<Line> {
    let (r1, r2) = (run(1), run(2));
    let (at, away) = (Some("lab/a"), Some("yard/b"));
    let mut script: Vec<Line> = vec![
        (RunId::CITY, None, EventKind::CityInitialized, json!({})),
        (r1, at, EventKind::RunStarted, json!({})),
        (r2, away, EventKind::RunStarted, json!({})),
    ];
    for n in 1..=commits {
        for k in 0..padding {
            let id = format!("p{n}-{k}");
            script.extend([
                (r2, away, EventKind::ToolCalled, called(&id, "read")),
                (r2, away, EventKind::ToolResult, answered(&id, "read")),
            ]);
        }
        let id = format!("c{n}");
        script.extend([
            (r1, at, EventKind::ModelCalled, attempt("m")),
            (r1, at, EventKind::ToolCalled, called(&id, "edit")),
            (r1, at, EventKind::ToolResult, answered(&id, "edit")),
            (
                r1,
                at,
                EventKind::CheckpointCommitted,
                json!({"oid": format!("{n:02x}").repeat(20), "scope": ["lab"], "files": []}),
            ),
        ]);
    }
    script
}

/// How many folds of the views one export of [`committing`] begins.
fn folds_exporting(commits: u8) -> u64 {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), &super::lines(committing(commits, 0)), b"");
    let before = crate::trace::counted::views_folds();
    let bundle = parsed(export(dir.path(), &person()).unwrap().bytes());
    let traced = bundle["checkpoints"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["holds"]["committed"]["trace"]["traced"].is_object())
        .count();
    assert_eq!(traced, usize::from(commits));
    crate::trace::counted::views_folds() - before
}

/// The regression this gate holds off is a commit's evidence folding
/// the whole history again: one export of twice the commits still
/// begins one fold of the views.
#[test]
fn an_export_folds_the_views_once_whatever_the_number_of_commits() {
    assert_eq!((folds_exporting(4), folds_exporting(8)), (1, 1));
}

/// The most index entries one commit's nearby walk takes in one export
/// of [`committing`], after checking that every commit walked once.
fn most_entries_one_walk_takes(commits: u8) -> u64 {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), &super::lines(committing(commits, 0)), b"");
    let before = crate::trace::counted::nearby_walks().len();
    export(dir.path(), &person()).unwrap();
    let walks = crate::trace::counted::nearby_walks();
    assert_eq!(walks.len() - before, usize::from(commits));
    walks[before..].iter().copied().max().unwrap()
}

/// The regression this gate holds off is a commit's nearby walk counting
/// the index from its first entry, which made one export cost commits
/// times lines. With twice the commits before it, the longest walk still
/// takes six entries: the first commit's span, five lines from its run's
/// first line, and the line after the span, which ends the walk.
#[test]
fn a_commits_nearby_walk_takes_its_span_whatever_came_before() {
    assert_eq!(
        (
            most_entries_one_walk_takes(4),
            most_entries_one_walk_takes(8)
        ),
        (6, 6)
    );
}

/// The milliseconds one export takes over [`committing`] histories of
/// growing length, printed for the register (`crates/accounting/spec/Playback/Traced.lean` §8-25).
#[test]
#[ignore = "an instrument: run it by name with --run-ignored only --no-capture"]
fn instrument_evidence_cost() {
    for commits in [50u8, 100, 200] {
        let dir = tempfile::tempdir().unwrap();
        let lines = super::lines(committing(commits, 8));
        write(dir.path(), &lines, b"");
        let started = std::time::Instant::now();
        let bundle = export(dir.path(), &person()).unwrap();
        let elapsed = started.elapsed();
        println!(
            "instrument_evidence_cost commits={commits} lines={} events={} export_ms={:.1}",
            lines.len(),
            bundle.events(),
            elapsed.as_secs_f64() * 1_000.0
        );
    }
}
