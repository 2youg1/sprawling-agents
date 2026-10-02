// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The lines a span of time keeps (`crates/sprawling/spec/Main.lean` §8-137): each line
//! judged by its own `t`, never by where it stands in the walk.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]

use super::tests::{grepped, run, viewed, write_city_ledger};
use super::{Audience, Selection};
use kernel::{Address, B3Hash, EventDraft, EventKind, EventRecord, Payload, Seq, TimeMs};
use serde_json::{Value, json};
use std::path::Path;

fn span(since: Option<u64>, until: Option<u64>) -> runtime::clock::UtcSpan {
    runtime::clock::UtcSpan::new(since.map(TimeMs::new), until.map(TimeMs::new)).unwrap()
}

/// The lines whose own `t` falls in `[since, until)`, alone and beside
/// another condition, are what grep and jq find in the same file.
#[test]
fn the_records_lens_keeps_the_lines_inside_a_time_window() {
    let dir = tempfile::tempdir().unwrap();
    let lines = write_city_ledger(dir.path());
    let t = |v: &Value| v["t"].as_u64().unwrap();
    let cases: Vec<(Selection, Vec<u8>)> = vec![
        (
            Selection {
                span: span(Some(3), Some(7)),
                ..Selection::default()
            },
            grepped(&lines, |v, _| (3..7).contains(&t(v)), usize::MAX),
        ),
        (
            Selection {
                span: span(Some(4), None),
                kind: Some(EventKind::ToolCalled),
                ..Selection::default()
            },
            grepped(
                &lines,
                |v, _| t(v) >= 4 && v["kind"] == "tool_called",
                usize::MAX,
            ),
        ),
        (
            Selection {
                span: span(None, Some(5)),
                tail: Some(2),
                ..Selection::default()
            },
            grepped(&lines, |v, _| t(v) < 5, 2),
        ),
    ];
    for (chosen, expected) in cases {
        assert_eq!(
            String::from_utf8(viewed(dir.path(), &chosen, Audience::Agent)).unwrap(),
            String::from_utf8(expected).unwrap(),
            "{chosen:?}"
        );
    }
}

/// A ledger whose `t` steps back and forth: one run's calls at the
/// given moments, chained from genesis.
fn write_moments(dir: &Path, moments: &[u64]) -> Vec<Vec<u8>> {
    let mut prev = kernel::GENESIS_PREV;
    let mut blob = Vec::new();
    let mut lines = Vec::new();
    for (seq, at) in moments.iter().enumerate() {
        let draft = EventDraft {
            run: run(1),
            t: TimeMs::new(*at),
            who: "tester".to_owned(),
            addr: Some(Address::parse("lab/a").unwrap()),
            kind: EventKind::ToolCalled,
            data: Payload::new(json!({"tool": "read"}).as_object().cloned().unwrap()).unwrap(),
            ig: false,
        };
        let line = EventRecord::from_draft(draft, Seq::new(u64::try_from(seq).unwrap()), prev)
            .canonical_line()
            .unwrap();
        prev = B3Hash::digest(&line);
        blob.extend_from_slice(&line);
        blob.push(b'\n');
        lines.push(line);
    }
    std::fs::write(dir.join("ledger-00000000000000000000.jsonl"), &blob).unwrap();
    lines
}

/// `t` does not rise with seq, so a line past `until` does not end the
/// walk: a later line that steps back into the window is still kept.
#[test]
fn a_line_whose_time_steps_back_is_judged_on_its_own() {
    let dir = tempfile::tempdir().unwrap();
    let lines = write_moments(dir.path(), &[10, 50, 20, 60, 30, 5, 15]);
    let kept = |chosen: &Selection| {
        String::from_utf8(viewed(dir.path(), chosen, Audience::Agent)).unwrap()
    };
    let joined = |seqs: &[usize]| {
        seqs.iter()
            .map(|seq| format!("{}\n", String::from_utf8(lines[*seq].clone()).unwrap()))
            .collect::<String>()
    };
    let window = Selection {
        span: span(Some(15), Some(40)),
        ..Selection::default()
    };
    assert_eq!(kept(&window), joined(&[2, 4, 6]));
    let last = Selection {
        tail: Some(1),
        span: span(Some(15), Some(40)),
        ..Selection::default()
    };
    assert_eq!(kept(&last), joined(&[6]));
}

/// A moment `view` cannot read, and a span with nothing in it, are
/// command-line errors that say what to write instead.
#[test]
fn a_moment_view_cannot_read_names_the_shape_it_wants() {
    let read = |words: &[&str]| {
        let words: Vec<String> = words.iter().map(|word| (*word).to_owned()).collect();
        match super::super::grammar::parse(&words) {
            Ok(super::super::grammar::Invocation::Run(_, arguments)) => Selection::read(&arguments),
            other => panic!("not a view line: {other:?}"),
        }
    };
    let unread = read(&["view", "city", "--since", "yesterday"]);
    assert!(
        matches!(&unread, Err(line) if line.starts_with("--since 'yesterday': ")
            && line.contains("2026-05-14T09:31:07Z")),
        "{unread:?}"
    );
    let empty = read(&[
        "view",
        "city",
        "--since",
        "2026-05-14T10:00:00Z",
        "--until",
        "2026-05-14T09:00:00Z",
    ]);
    assert!(
        matches!(&empty, Err(line) if line.starts_with("--since and --until: ")),
        "{empty:?}"
    );
    let both = read(&[
        "view",
        "city",
        "--since",
        "2026-05-14T09:00:00Z",
        "--until",
        "2026-05-14T10:00:00Z",
    ]);
    assert!(
        matches!(&both, Ok(chosen) if chosen.span == span(Some(1_778_749_200_000), Some(1_778_752_800_000))),
        "{both:?}"
    );
}
