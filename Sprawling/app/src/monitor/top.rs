// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::Sample;

pub fn json_line(sample: &Sample) -> serde_json::Result<String> {
    serde_json::to_string(sample)
}

const LEVELS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

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

fn level(offset: u64, span: u64) -> Option<char> {
    let top = u128::try_from(LEVELS.len().checked_sub(1)?).ok()?;
    let rank = u128::from(offset)
        .checked_mul(top)?
        .checked_div(u128::from(span))
        .unwrap_or(0);
    LEVELS.get(usize::try_from(rank).ok()?).copied()
}

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

struct Row {
    label: &'static str,
    read: fn(&Sample) -> u64,
    unit: Unit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    Permille,
    Bytes,
    Nanos,
    Count,
}

const ROWS: [Row; 15] = [
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
    Row {
        label: "view backlog",
        read: |s| s.view_backlog,
        unit: Unit::Count,
    },
    Row {
        label: "monitor read",
        read: |s| s.read_nanos,
        unit: Unit::Nanos,
    },
];

impl Unit {
    #[must_use]
    pub fn reading(&self, value: u64) -> String {
        match self {
            Self::Permille => format!("{}%", tenths(u128::from(value))),
            Self::Bytes => scaled(value, 1024, &["B", "KiB", "MiB", "GiB", "TiB"]),
            Self::Nanos => duration(value),
            Self::Count => value.to_string(),
        }
    }
}

fn duration(nanos: u64) -> String {
    match nanos {
        0..1_000 => format!("{nanos} ns"),
        1_000..10_000_000 => format!("{} µs", nanos.checked_div(1_000).unwrap_or(0)),
        10_000_000.. => format!(
            "{} ms",
            tenths(u128::from(nanos.checked_div(100_000).unwrap_or(0)))
        ),
    }
}

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
