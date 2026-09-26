// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One JSON line carries every counter; the curve scales to its window.

#![allow(clippy::unwrap_used, reason = "test code")]

use super::super::Sample;
use super::{json_line, sparkline};

#[test]
fn a_json_line_carries_every_counter_under_its_field_name() {
    let sample = Sample {
        core_cpu_permille: 7,
        queued_runs: 3,
        volume_free_bytes: u64::MAX,
        ..Sample::default()
    };
    let line = json_line(&sample).unwrap();
    let object: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&line).unwrap();
    assert_eq!(
        (
            line.contains('\n'),
            object.len(),
            object["core_cpu_permille"].as_u64(),
            object["queued_runs"].as_u64(),
            object["volume_free_bytes"].as_u64(),
        ),
        (false, 13, Some(7), Some(3), Some(u64::MAX)),
    );
}

#[test]
fn a_sparkline_scales_the_latest_window_from_its_minimum_to_its_maximum() {
    assert_eq!(
        (
            sparkline(0..8, 8),
            sparkline([100, 0, 7, 14], 2),
            sparkline([5, 5, 5], 10),
            sparkline(Vec::new(), 4),
        ),
        (
            "▁▂▃▄▅▆▇█".to_owned(),
            "▁█".to_owned(),
            "▁▁▁".to_owned(),
            String::new(),
        ),
    );
}
