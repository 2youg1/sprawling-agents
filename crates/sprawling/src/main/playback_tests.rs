// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use accounting::playback::{Report, Verdict};
use kernel::B3Hash;
use serde_json::json;

use super::super::exit::Exit;
use super::super::grammar::{Invocation, parse};
use super::super::verbs::Verb;
use super::{Refused, outcome, request_of};

/// How `playback export` refuses `line`, the verb given as the one word
/// its row is named, so the grammar's two-word reading is not what this
/// judges.
fn refusal_of(line: &[&str]) -> Option<&'static str> {
    let words: Vec<String> = line.iter().map(|word| (*word).to_owned()).collect();
    let Ok(Invocation::Run(Verb::PlaybackExport, read)) = parse(&words) else {
        panic!("{line:?} did not parse as playback export");
    };
    match request_of(&read) {
        Ok(_) => None,
        Err(Refused::Line(_)) => Some("line"),
        Err(Refused::Selection(_)) => Some("selection"),
    }
}

#[test]
fn a_range_or_a_building_that_does_not_read_is_a_command_line_refusal() {
    assert_eq!(
        [
            refusal_of(&["playback export", "c", "--from", "5", "--through", "2"]),
            refusal_of(&["playback export", "c", "--building", "a:b"]),
            refusal_of(&["playback export", "c", "--from", "two"]),
            refusal_of(&["playback export", "c", "--from", "2", "--through", "5"]),
        ],
        [Some("selection"), Some("line"), Some("line"), None]
    );
}

/// A report whose items are as given, about a three-event bundle.
fn report(source: Verdict, browser: Verdict) -> Report {
    Report {
        digest: Some(B3Hash::digest(b"bundle")),
        events: Some(3),
        structure: Verdict::Passed,
        bundle: Verdict::Unasked {
            why: "no other bundle was given",
        },
        source,
        offline: Verdict::Passed,
        browser,
        covered: vec!["load".to_owned()],
    }
}

#[test]
fn a_check_exits_refused_on_a_failed_item_or_one_asked_for_and_not_done() {
    let digest = B3Hash::digest(b"bundle").to_string();
    let unasked = || Verdict::Unasked {
        why: "no browser observation was given; the product does not run the page",
    };
    assert_eq!(
        [
            outcome(&report(Verdict::Passed, Verdict::Passed)),
            outcome(&report(
                Verdict::Failed {
                    found: "differs from its recomputation first in `costs`".to_owned()
                },
                unasked()
            )),
            outcome(&report(
                Verdict::Unable {
                    why: "the city's ledger ends at seq 1, before the cutoff 4".to_owned()
                },
                unasked()
            )),
        ],
        [
            (
                json!({
                    "digest": digest,
                    "events": "3",
                    "structure": {"status": "passed"},
                    "bundle": {"status": "unchecked", "why": "no other bundle was given"},
                    "source": {"status": "passed"},
                    "offline": {"status": "passed"},
                    "browser": {"status": "passed", "covered": ["load"]},
                }),
                Exit::Done
            ),
            (
                json!({
                    "digest": digest,
                    "events": "3",
                    "structure": {"status": "passed"},
                    "bundle": {"status": "unchecked", "why": "no other bundle was given"},
                    "source": {"status": "failed", "found": "differs from its recomputation first in `costs`"},
                    "offline": {"status": "passed"},
                    "browser": {"status": "unchecked", "why": "no browser observation was given; the product does not run the page"},
                }),
                Exit::Refused
            ),
            (
                json!({
                    "digest": digest,
                    "events": "3",
                    "structure": {"status": "passed"},
                    "bundle": {"status": "unchecked", "why": "no other bundle was given"},
                    "source": {"status": "unchecked", "why": "the city's ledger ends at seq 1, before the cutoff 4"},
                    "offline": {"status": "passed"},
                    "browser": {"status": "unchecked", "why": "no browser observation was given; the product does not run the page"},
                }),
                Exit::Refused
            ),
        ]
    );
}
