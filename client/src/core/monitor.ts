// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The performance panel's reading of the monitor history: a row per
// counter with its latest reading, its window's p50 and p99, and its
// bars. The readings are written by the unit rule `sprawling top` uses
// (`crates/sprawling/spec/Monitor.lean` §8-95), so a User at the page
// and an agent at the terminal read one sample as the same figure;
// `monitor.test.ts` holds this to the fixture `bin::monitor::top`'s own
// screen test uses. The bars are the page's own drawing (D82 in
// `client/Spec.lean`): the terminal has eight glyph heights, the page
// has as many pixels as it gives a plot.

import type { Sample } from "../wire";
import type { Key } from "./lang";

export interface Row {
  readonly label: Key;
  readonly reading: string;
  // The window's p50 and p99 by nearest rank, in the counter's unit.
  readonly p50: string;
  readonly p99: string;
  readonly plot: Plot;
}

// A row per counter over the last `width` samples; none before the first sample.
export function rows(samples: readonly Sample[], width: number): readonly Row[] {
  const latest = samples.at(-1);
  if (latest === undefined) {
    return [];
  }
  const window = samples.slice(Math.max(samples.length - width, 0));
  return COUNTERS.map(({ label, field, unit }) => {
    const values = window.map((sample) => sample[field]);
    return {
      label,
      reading: reading(unit, latest[field]),
      p50: reading(unit, nearestRank(values, 50) ?? latest[field]),
      p99: reading(unit, nearestRank(values, 99) ?? latest[field]),
      plot: plot(values),
    };
  });
}

// The fact bar's summary of the latest reading: this process's CPU
// share and working set, the two figures a summary-only watch reads.
export function summary(latest: Sample): { readonly cpu: string; readonly memory: string } {
  return {
    cpu: reading("permille", latest.core_cpu_permille),
    memory: reading("bytes", latest.core_working_set_bytes),
  };
}

// A plot's heights, in thousandths of the height the page gives it.
export interface Plot {
  // One bar per value, oldest first.
  readonly bars: readonly number[];
  // Where the window's p50 and p99 stand, the band a bar is read against.
  readonly p50: number;
  readonly p99: number;
}

// The full height of a plot, and the height of the window's lowest bar.
export const FULL = 1000;
const FLOOR = 80;

// The values as bars scaled from the window's own minimum (`FLOOR`, so
// the lowest bar still shows) to its maximum (`FULL`); a flat window
// stands at half height. The scale is linear from the minimum rather
// than from zero on a linear or a logarithmic scale, because the panel
// watches change: a working set moving between 3.1 and 3.2 GiB draws a
// flat line on both of those (D82). An empty window has no bars and no band.
export function plot(values: readonly number[]): Plot {
  if (values.length === 0) {
    return { bars: [], p50: 0, p99: 0 };
  }
  const low = Math.min(...values);
  const span = Math.max(...values) - low;
  const height = (value: number): number =>
    span === 0 ? FULL / 2 : FLOOR + Math.floor(((value - low) * (FULL - FLOOR)) / span);
  return {
    bars: values.map(height),
    p50: height(nearestRank(values, 50) ?? low),
    p99: height(nearestRank(values, 99) ?? low),
  };
}

// The `percent`th percentile by nearest rank, the rank rule
// `bin::monitor::spread::Spread` reads with; none of an empty window.
function nearestRank(values: readonly number[], percent: number): number | undefined {
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.max(Math.ceil((percent * sorted.length) / 100) - 1, 0)];
}

type Unit = "permille" | "bytes" | "nanos" | "count";

interface Counter {
  readonly label: Key;
  readonly field: keyof Sample;
  readonly unit: Unit;
}

// The counters in `Sample` field order, as the terminal lists them.
const COUNTERS: readonly Counter[] = [
  { label: "monitor_core_cpu", field: "core_cpu_permille", unit: "permille" },
  { label: "monitor_core_private", field: "core_private_bytes", unit: "bytes" },
  { label: "monitor_core_working_set", field: "core_working_set_bytes", unit: "bytes" },
  { label: "monitor_core_read", field: "core_read_bytes", unit: "bytes" },
  { label: "monitor_core_written", field: "core_written_bytes", unit: "bytes" },
  { label: "monitor_machine_cpu", field: "machine_cpu_permille", unit: "permille" },
  { label: "monitor_machine_available", field: "machine_available_bytes", unit: "bytes" },
  { label: "monitor_volume_free", field: "volume_free_bytes", unit: "bytes" },
  { label: "monitor_ledger_queue_depth", field: "ledger_queue_depth", unit: "count" },
  { label: "monitor_durable_lag", field: "durable_lag", unit: "count" },
  { label: "monitor_relay_p50", field: "relay_p50_nanos", unit: "nanos" },
  { label: "monitor_event_to_screen_p50", field: "event_to_screen_p50_nanos", unit: "nanos" },
  { label: "monitor_queued_runs", field: "queued_runs", unit: "count" },
];

// `value` in `unit`, one decimal truncated where a larger unit applies.
function reading(unit: Unit, value: number): string {
  switch (unit) {
    case "permille":
      return `${tenths(value)}%`;
    case "bytes":
      return scaled(value, 1024, ["B", "KiB", "MiB", "GiB", "TiB"]);
    case "nanos":
      return duration(value);
    case "count":
      return String(value);
  }
}

// A duration in nanoseconds as a person reads it: below 1 µs in ns,
// below 10 ms in whole µs, from 10 ms in ms with one decimal, truncated
// (`crates/sprawling/spec/Main.lean` §8-129-2, the unit rule).
function duration(nanos: number): string {
  if (nanos < 1_000) {
    return `${String(nanos)} ns`;
  }
  if (nanos < 10_000_000) {
    return `${String(Math.floor(nanos / 1_000))} µs`;
  }
  return `${tenths(Math.floor(nanos / 100_000))} ms`;
}

// `value` in the largest of `units` (each `base` times the one before)
// that it reaches at least 1 of; the first unit is written whole.
function scaled(value: number, base: number, units: readonly string[]): string {
  const rank = units.reduce(
    (reached, _, index) => (index > 0 && value >= base ** index ? index : reached),
    0,
  );
  const unit = units[rank] ?? "";
  return rank === 0
    ? `${String(value)} ${unit}`
    : `${tenths(Math.floor((value * 10) / base ** rank))} ${unit}`;
}

// A count of tenths as a number with one decimal, 123 as 12.3.
function tenths(count: number): string {
  return `${String(Math.floor(count / 10))}.${String(count % 10)}`;
}
