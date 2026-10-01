// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One JSON line carries every counter; the curve scales to its window.

#![allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "test code")]

use super::super::Sample;
use super::{json_line, screen, sparkline};

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
        (false, 15, Some(7), Some(3), Some(u64::MAX)),
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

fn row(label: &str, reading: &str, curve: &str) -> String {
    format!("{label:<24}{reading:>10}  {curve}")
}

#[test]
fn a_screen_shows_each_counter_in_its_unit_beside_its_curve() {
    const GIB: u64 = 1 << 30;
    let before = Sample {
        core_cpu_permille: 100,
        core_private_bytes: 3 * GIB,
        core_working_set_bytes: 512,
        core_read_bytes: 0,
        core_written_bytes: 1536,
        machine_cpu_permille: 999,
        machine_available_bytes: 8 * GIB,
        volume_free_bytes: 1 << 40,
        ledger_queue_depth: 0,
        durable_lag: 2,
        relay_p50_nanos: 900,
        event_to_screen_p50_nanos: 16_600_000,
        queued_runs: 1,
        view_backlog: 0,
        read_nanos: 2_000,
    };
    let after = Sample {
        core_cpu_permille: 123,
        core_private_bytes: 3_382_286_746,
        durable_lag: 5,
        relay_p50_nanos: 1_500_000,
        queued_runs: 4,
        view_backlog: 300,
        ..before
    };
    let expected = [
        row("core cpu", "12.3%", "▁█"),
        row("core private", "3.1 GiB", "▁█"),
        row("core working set", "512 B", "▁▁"),
        row("core read", "0 B", "▁▁"),
        row("core written", "1.5 KiB", "▁▁"),
        row("machine cpu", "99.9%", "▁▁"),
        row("machine available", "8.0 GiB", "▁▁"),
        row("volume free", "1.0 TiB", "▁▁"),
        row("ledger queue depth", "0", "▁▁"),
        row("durable lag", "5", "▁█"),
        row("relay p50", "1.5 ms", "▁█"),
        row("event to screen p50", "16.6 ms", "▁▁"),
        row("queued runs", "4", "▁█"),
        row("view backlog", "300", "▁█"),
        row("monitor read", "2.0 µs", "▁▁"),
    ]
    .join(
        "
",
    );
    assert_eq!(
        (screen(&[before, after], 8), screen(&[], 8)),
        (expected, String::new()),
    );
}
