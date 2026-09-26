// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The performance panel's reading of the monitor history: a row per
// counter with its latest reading and its curve, drawn exactly as
// `sprawling top` draws them (sprawling-SPEC 8-91), so a person at the
// page and an agent at the terminal read one sample the same way.
// `monitor.test.ts` holds this to the fixture `bin::monitor::top`'s own
// screen test uses.

import type { Sample } from "../wire";
import type { Key } from "./lang";

export interface Row {
  readonly label: Key;
  readonly reading: string;
  readonly curve: string;
}

// A row per counter; none before the first sample.
export function rows(samples: readonly Sample[], width: number): readonly Row[] {
  const latest = samples.at(-1);
  if (latest === undefined) {
    return [];
  }
  return COUNTERS.map(({ label, field, unit }) => ({
    label,
    reading: reading(unit, latest[field]),
    curve: sparkline(
      samples.map((sample) => sample[field]),
      width,
    ),
  }));
}

// The fact bar's summary of the latest reading: this process's CPU
// share and working set, the two figures a summary-only watch reads.
export function summary(latest: Sample): { readonly cpu: string; readonly memory: string } {
  return {
    cpu: reading("permille", latest.core_cpu_permille),
    memory: reading("bytes", latest.core_working_set_bytes),
  };
}

// The eight heights a curve is drawn in, lowest first.
const LEVELS = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];

// The last `width` values as one block each, scaled from the window's
// own minimum (lowest block) to its maximum (highest); a flat window
// draws the lowest block.
export function sparkline(values: readonly number[], width: number): string {
  const window = values.slice(Math.max(values.length - width, 0));
  const low = Math.min(...window);
  const span = Math.max(...window) - low;
  const top = LEVELS.length - 1;
  return window
    .map((value) => LEVELS[span === 0 ? 0 : Math.floor(((value - low) * top) / span)] ?? "")
    .join("");
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
      return scaled(value, 1000, ["ns", "µs", "ms", "s"]);
    case "count":
      return String(value);
  }
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
