// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Every line `gauge` writes, and the one place each becomes text
//! (`crates/sprawling/spec/Main.lean` §8-129-4).
//!
//! For an agent a line is one JSON object: the `line` key first, then
//! integers under keys that end in their unit, `null` where nothing was
//! measured, in a fixed order. For a person it is the same reading in
//! units, through the one unit table, `monitor::top::Unit`.

use std::fmt::Display;
use std::time::Duration;

use sprawling::audience::Audience;
use sprawling::monitor::Sample;
use sprawling::monitor::spread::{Share, Spread};
use sprawling::monitor::top::Unit;
use sprawling::monitor::tree::TreeReading;

use super::{Host, Run};

/// What a person sees where nothing was measured.
const UNMEASURED: &str = "-";

/// One reading of a served city: `{"line":"city"}` followed by the
/// sample's own fields, in their order.
///
/// # Errors
/// Only what `serde_json` reports; a struct of integers gives it nothing
/// to refuse.
pub(crate) fn city_line(sample: &Sample) -> serde_json::Result<String> {
    #[derive(serde::Serialize)]
    struct CityLine<'sample> {
        line: &'static str,
        #[serde(flatten)]
        sample: &'sample Sample,
    }
    serde_json::to_string(&CityLine {
        line: "city",
        sample,
    })
}

/// One beat of a process tree, `at` after the watch began.
pub(super) fn tree_line(at: Duration, reading: &TreeReading, audience: Audience) -> String {
    match audience {
        Audience::Agent => object(
            "tree",
            &[
                ("at_us", micros(at).to_string()),
                ("processes", reading.processes.to_string()),
                ("cpu_permille", or_null(reading.cpu_permille)),
                ("private_bytes", reading.private_bytes.to_string()),
                ("working_set_bytes", reading.working_set_bytes.to_string()),
                ("read_bytes", reading.read_bytes.to_string()),
                ("written_bytes", reading.written_bytes.to_string()),
            ],
        ),
        Audience::Person => format!(
            "{:>10}  {} process(es)  cpu {}  private {}  working set {}  read {}  written {}",
            Unit::Nanos.reading(nanos(at)),
            reading.processes,
            in_unit(Unit::Permille, reading.cpu_permille),
            Unit::Bytes.reading(reading.private_bytes),
            Unit::Bytes.reading(reading.working_set_bytes),
            Unit::Bytes.reading(reading.read_bytes),
            Unit::Bytes.reading(reading.written_bytes),
        ),
    }
}

/// One run of a measured command.
pub(super) fn run_line(run: &Run, audience: Audience) -> String {
    let seen = run.watched.seen;
    let seen_of = |figure: fn(&sprawling::monitor::tree::Seen) -> u64| seen.as_ref().map(figure);
    match audience {
        Audience::Agent => object(
            "run",
            &[
                ("index", run.index.to_string()),
                ("exit", or_null(run.exit)),
                ("wall_us", micros(run.wall).to_string()),
                ("beats", run.watched.beats.to_string()),
                ("seen_cpu_ms", or_null(seen_of(|seen| seen.cpu_ms))),
                ("seen_read_bytes", or_null(seen_of(|seen| seen.read_bytes))),
                (
                    "seen_written_bytes",
                    or_null(seen_of(|seen| seen.written_bytes)),
                ),
                (
                    "seen_peak_private_bytes",
                    or_null(seen_of(|seen| seen.peak_private_bytes)),
                ),
                (
                    "seen_peak_working_set_bytes",
                    or_null(seen_of(|seen| seen.peak_working_set_bytes)),
                ),
                ("child_peak_private_bytes", or_null(run.child.private_bytes)),
                (
                    "child_peak_working_set_bytes",
                    or_null(run.child.working_set_bytes),
                ),
                ("read_cost_us", micros(run.watched.read_cost).to_string()),
            ],
        ),
        Audience::Person => format!(
            "run {}  exit {}  {}  {} beat(s)  cpu {}  peak private {}  peak working set {}  child peak private {}  child peak working set {}  read cost {}",
            run.index,
            run.exit
                .map_or_else(|| "by signal".to_owned(), |code| code.to_string()),
            Unit::Nanos.reading(nanos(run.wall)),
            run.watched.beats,
            in_unit(
                Unit::Nanos,
                seen_of(|seen| seen.cpu_ms.saturating_mul(1_000_000))
            ),
            in_unit(Unit::Bytes, seen_of(|seen| seen.peak_private_bytes)),
            in_unit(Unit::Bytes, seen_of(|seen| seen.peak_working_set_bytes)),
            in_unit(Unit::Bytes, run.child.private_bytes),
            in_unit(Unit::Bytes, run.child.working_set_bytes),
            Unit::Nanos.reading(nanos(run.watched.read_cost)),
        ),
    }
}

/// The spread of every run's wall time, `failed` of them non-zero, on
/// a machine of `host`'s class.
pub(super) fn spread_line(spread: &Spread, failed: u32, host: Host, audience: Audience) -> String {
    match audience {
        Audience::Agent => object(
            "spread",
            &[
                ("samples", spread.samples().to_string()),
                ("failed", failed.to_string()),
                ("floor_us", micros(spread.floor()).to_string()),
                ("p50_us", micros(spread.p(Share::P50)).to_string()),
                ("p95_us", micros(spread.p(Share::P95)).to_string()),
                ("p99_us", micros(spread.p(Share::P99)).to_string()),
                ("peak_us", micros(spread.peak()).to_string()),
                ("suspicious", spread.suspicious().to_string()),
                ("cores", host.cores.to_string()),
                ("physical_bytes", host.physical_bytes.to_string()),
            ],
        ),
        Audience::Person => format!(
            "{} run(s), {failed} failed  floor {}  p50 {}  p95 {}  p99 {}  peak {}  {} suspicious  on {} cores, {} memory",
            spread.samples(),
            Unit::Nanos.reading(nanos(spread.floor())),
            Unit::Nanos.reading(nanos(spread.p(Share::P50))),
            Unit::Nanos.reading(nanos(spread.p(Share::P95))),
            Unit::Nanos.reading(nanos(spread.p(Share::P99))),
            Unit::Nanos.reading(nanos(spread.peak())),
            spread.suspicious(),
            host.cores,
            Unit::Bytes.reading(host.physical_bytes),
        ),
    }
}

/// `{"line":"<line>","key":value,...}`. Every key is a constant here
/// and every value an integer or `null`, so nothing needs escaping.
fn object(line: &str, fields: &[(&str, String)]) -> String {
    let body: String = fields
        .iter()
        .map(|(key, value)| format!(",\"{key}\":{value}"))
        .collect();
    format!("{{\"line\":\"{line}\"{body}}}")
}

fn or_null(value: Option<impl Display>) -> String {
    value.map_or_else(|| "null".to_owned(), |value| value.to_string())
}

fn in_unit(unit: Unit, value: Option<u64>) -> String {
    value.map_or_else(|| UNMEASURED.to_owned(), |value| unit.reading(value))
}

fn micros(span: Duration) -> u64 {
    u64::try_from(span.as_micros()).unwrap_or(u64::MAX)
}

fn nanos(span: Duration) -> u64 {
    u64::try_from(span.as_nanos()).unwrap_or(u64::MAX)
}
