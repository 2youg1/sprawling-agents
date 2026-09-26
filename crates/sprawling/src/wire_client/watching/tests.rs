// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A reading becomes a JSON line or a redrawn screen; nothing else shows.

#![allow(clippy::unwrap_used, reason = "test code")]

use std::collections::VecDeque;

use sprawling::monitor::Sample;
use sprawling::monitor::top::{json_line, screen};

use super::{Output, shown};

fn reading(n: u64) -> (Sample, String) {
    let sample = Sample {
        queued_runs: n,
        ..Sample::default()
    };
    let text = serde_json::to_string(&channels::ServerFrame::Monitor(sample)).unwrap();
    (sample, text)
}

#[test]
fn a_reading_is_a_json_line_or_a_redrawn_screen_and_nothing_else_shows() {
    let (first, first_text) = reading(1);
    let (second, second_text) = reading(2);
    let mut history = VecDeque::new();

    let other = shown(r#"{"not":"a frame"}"#, &mut history, Output::Lines, 8);
    let kept_after_other = history.len();
    let line = shown(&first_text, &mut history, Output::Lines, 8);
    let redrawn = shown(&second_text, &mut history, Output::Screen, 8);

    let expected_screen = format!("\u{1b}[H\u{1b}[2J{}\n", screen(&[first, second], 8));
    assert_eq!(
        (other, kept_after_other, line, redrawn),
        (
            None,
            0,
            Some(format!("{}\n", json_line(&first).unwrap())),
            Some(expected_screen),
        )
    );
}
