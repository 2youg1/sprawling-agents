// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What `sprawling top` prints: one JSON line per sample when stdout is
//! not a terminal, and one curve per counter when it is
//! (sprawling-SPEC.md 8-95). Pure: the caller owns the terminal.

use super::Sample;

/// The line printed each second when stdout is not a terminal: one JSON
/// object keyed by the [`Sample`] field names, without the newline.
///
/// # Errors
///
/// Only what `serde_json` reports; a struct of integers gives it nothing
/// to refuse, and the caller treats it as a failed write to stdout.
pub fn json_line(sample: &Sample) -> serde_json::Result<String> {
    serde_json::to_string(sample)
}

/// The eight heights a curve is drawn in, lowest first.
const LEVELS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// The last `width` values as one block character each, scaled from the
/// window's own minimum (`▁`) to its maximum (`█`); a flat window is `▁`.
#[must_use]
pub fn sparkline(values: impl IntoIterator<Item = u64>, width: usize) -> String {
    let all: Vec<u64> = values.into_iter().collect();
    let window = all.iter().copied().skip(all.len().saturating_sub(width));
    let low = window.clone().min().unwrap_or(0);
    let span = window.clone().max().unwrap_or(0).saturating_sub(low);
    window
        .filter_map(|value| level(value.saturating_sub(low), span))
        .collect()
}

/// The character `offset` above the window's minimum reaches in a window
/// `span` tall. A flat window has no height to divide, so it draws the
/// lowest level; `offset <= span` keeps the rank inside [`LEVELS`].
fn level(offset: u64, span: u64) -> Option<char> {
    let top = u128::try_from(LEVELS.len().checked_sub(1)?).ok()?;
    let rank = u128::from(offset)
        .checked_mul(top)?
        .checked_div(u128::from(span))
        .unwrap_or(0);
    LEVELS.get(usize::try_from(rank).ok()?).copied()
}

/// One terminal screen: a row per counter with its label, its latest
/// reading and its last `curve_width` points; empty with no samples.
#[must_use]
pub fn screen(samples: &[Sample], curve_width: usize) -> String {
    let Some(latest) = samples.last() else {
        return String::new();
    };
    ROWS.iter()
        .map(|row| {
            let label = row.label;
            let reading = row.unit.reading((row.read)(latest));
            let curve = sparkline(samples.iter().map(row.read), curve_width);
            format!("{label:<24}{reading:>10}  {curve}")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// One counter as the screen shows it.
struct Row {
    label: &'static str,
    read: fn(&Sample) -> u64,
    unit: Unit,
}

/// What a counter's integer counts, which decides how it is written.
enum Unit {
    Permille,
    Bytes,
    Nanos,
    Count,
}

/// The counters in [`Sample`] field order.
const ROWS: [Row; 13] = [
    Row {
        label: "core cpu",
        read: |s| s.core_cpu_permille,
        unit: Unit::Permille,
    },
    Row {
        label: "core private",
        read: |s| s.core_private_bytes,
        unit: Unit::Bytes,
    },
    Row {
        label: "core working set",
        read: |s| s.core_working_set_bytes,
        unit: Unit::Bytes,
    },
    Row {
        label: "core read",
        read: |s| s.core_read_bytes,
        unit: Unit::Bytes,
    },
    Row {
        label: "core written",
        read: |s| s.core_written_bytes,
        unit: Unit::Bytes,
    },
    Row {
        label: "machine cpu",
        read: |s| s.machine_cpu_permille,
        unit: Unit::Permille,
    },
    Row {
        label: "machine available",
        read: |s| s.machine_available_bytes,
        unit: Unit::Bytes,
    },
    Row {
        label: "volume free",
        read: |s| s.volume_free_bytes,
        unit: Unit::Bytes,
    },
    Row {
        label: "ledger queue depth",
        read: |s| s.ledger_queue_depth,
        unit: Unit::Count,
    },
    Row {
        label: "durable lag",
        read: |s| s.durable_lag,
        unit: Unit::Count,
    },
    Row {
        label: "relay p50",
        read: |s| s.relay_p50_nanos,
        unit: Unit::Nanos,
    },
    Row {
        label: "event to screen p50",
        read: |s| s.event_to_screen_p50_nanos,
        unit: Unit::Nanos,
    },
    Row {
        label: "queued runs",
        read: |s| s.queued_runs,
        unit: Unit::Count,
    },
];

impl Unit {
    /// `value` in this unit, one decimal truncated where a larger unit
    /// applies.
    fn reading(&self, value: u64) -> String {
        match self {
            Self::Permille => format!("{}%", tenths(u128::from(value))),
            Self::Bytes => scaled(value, 1024, &["B", "KiB", "MiB", "GiB", "TiB"]),
            Self::Nanos => scaled(value, 1000, &["ns", "µs", "ms", "s"]),
            Self::Count => value.to_string(),
        }
    }
}

/// `value` in the largest of `units` (each `base` times the one before)
/// that it reaches at least 1 of; the first unit is written whole.
fn scaled(value: u64, base: u64, units: &[&str]) -> String {
    let value = u128::from(value);
    let (divisor, unit) = units
        .iter()
        .scan(1_u128, |divisor, &unit| {
            let this = *divisor;
            *divisor = divisor.saturating_mul(u128::from(base));
            Some((this, unit))
        })
        .take_while(|&(divisor, _)| divisor == 1 || value >= divisor)
        .last()
        .unwrap_or((1, ""));
    if divisor == 1 {
        format!("{value} {unit}")
    } else {
        let whole_tenths = value.saturating_mul(10).checked_div(divisor).unwrap_or(0);
        format!("{} {unit}", tenths(whole_tenths))
    }
}

/// A count of tenths as a number with one decimal, `123` as `12.3`.
fn tenths(count: u128) -> String {
    format!(
        "{}.{}",
        count.checked_div(10).unwrap_or(0),
        count.checked_rem(10).unwrap_or(0)
    )
}

#[cfg(test)]
#[path = "top/tests.rs"]
mod tests;
