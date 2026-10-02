// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A selection by UTC time (`crates/accounting/spec/Playback/Traced.lean` §8-17): judged on each
//! line's own `t`, a day crossed with `since` and `until`, and a span
//! no line falls in exported as an empty bundle that names its ends.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use kernel::{AxCode, EventKind, RunId, TimeMs};
use runtime::clock::UtcSpan;
use serde_json::json;

use super::super::{Asked, City, Selection, Verdict, Window, check, export};
use super::{city, lines_at, parsed, person, run, seqs, write};

fn span(since: Option<u64>, until: Option<u64>) -> UtcSpan {
    UtcSpan::new(since.map(TimeMs::new), until.map(TimeMs::new)).unwrap()
}

/// Seq 3 is written after seq 2 but records an earlier moment; seq 4
/// is past the end of the span and seq 5 steps back inside it again.
#[test]
fn a_line_whose_time_steps_back_is_judged_on_its_own() {
    let dir = tempfile::tempdir().unwrap();
    let call = || json!({"tool": "read"});
    let written = lines_at(vec![
        (
            0,
            (RunId::CITY, None, EventKind::CityInitialized, json!({})),
        ),
        (
            10,
            (run(1), Some("lab/a"), EventKind::RunStarted, json!({})),
        ),
        (30, (run(1), Some("lab/a"), EventKind::ToolCalled, call())),
        (20, (run(1), Some("lab/a"), EventKind::ToolCalled, call())),
        (40, (run(1), Some("lab/a"), EventKind::ToolCalled, call())),
        (25, (run(1), Some("lab/a"), EventKind::ToolCalled, call())),
    ]);
    write(dir.path(), &written, b"");
    let chosen = super::super::Request {
        selection: Selection::everything().during(span(Some(20), Some(40))),
        ..person()
    };
    let bundle = parsed(export(dir.path(), &chosen).unwrap().bytes());
    assert_eq!(seqs(&bundle["events"]), ["2", "3", "5"]);
}

/// The span three conditions make, or how it is refused.
fn spanned(since: Option<&str>, until: Option<&str>, day: Option<&str>) -> Result<UtcSpan, AxCode> {
    Window { since, until, day }
        .span()
        .map_err(|err| *err.code())
}

#[test]
fn a_day_crosses_since_and_until_and_a_crossing_that_holds_nothing_is_refused() {
    let day = 86_400_000;
    assert_eq!(
        [
            spanned(None, None, Some("1970-01-02")),
            spanned(Some("1970-01-02T09:00:00Z"), None, Some("1970-01-02")),
            spanned(None, Some("1970-01-05T00:00:00Z"), Some("1970-01-02")),
            spanned(Some("1970-01-03T00:00:00Z"), None, Some("1970-01-02")),
            spanned(None, None, Some("1970-02-30")),
            spanned(None, None, Some("1970-01-02T00:00:00Z")),
            spanned(Some("1970-01-02T09:00:00"), None, None),
            spanned(None, None, None),
        ],
        [
            Ok(span(Some(day), Some(2 * day))),
            Ok(span(Some(day + 9 * 3_600_000), Some(2 * day))),
            Ok(span(Some(day), Some(2 * day))),
            Err(AxCode::InvalidArgs),
            Err(AxCode::InvalidArgs),
            Err(AxCode::InvalidArgs),
            Err(AxCode::InvalidArgs),
            Ok(UtcSpan::default()),
        ]
    );
}

/// A span after every line selects nothing; the bundle still names the
/// span's two ends, and recomputing it from the city gives it back.
#[test]
fn a_span_no_line_falls_in_is_an_empty_bundle_that_names_its_ends() {
    let (dir, _) = city();
    let chosen = super::super::Request {
        selection: Selection::everything().during(span(Some(1_000), Some(2_000))),
        ..person()
    };
    let bundle = export(dir.path(), &chosen).unwrap();
    let read = parsed(bundle.bytes());
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
    assert_eq!(
        (
            read["events"].clone(),
            read["source"]["selection"].clone(),
            recomputed.source,
        ),
        (
            json!([]),
            json!({
                "from": null, "through": null, "run": null, "building": null,
                "since": "1000", "until": "2000",
            }),
            Verdict::Passed,
        )
    );
}
