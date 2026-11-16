// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Sample } from "../wire";
import type { Key } from "./lang";

export interface Row {
  readonly label: Key;
  readonly reading: string;

  readonly p50: string;
  readonly p99: string;
  readonly plot: Plot;
}

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

export function summary(latest: Sample): { readonly cpu: string; readonly memory: string } {
  return {
    cpu: reading("permille", latest.core_cpu_permille),
    memory: reading("bytes", latest.core_working_set_bytes),
  };
}

export interface Plot {

  readonly bars: readonly number[];

  readonly p50: number;
  readonly p99: number;
}

export const FULL = 1000;
const FLOOR = 80;

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

function duration(nanos: number): string {
  if (nanos < 1_000) {
    return `${String(nanos)} ns`;
  }
  if (nanos < 10_000_000) {
    return `${String(Math.floor(nanos / 1_000))} µs`;
  }
  return `${tenths(Math.floor(nanos / 100_000))} ms`;
}

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

function tenths(count: number): string {
  return `${String(Math.floor(count / 10))}.${String(count % 10)}`;
}
