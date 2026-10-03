// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The panel reads what `sprawling top` reads: the same fixture as
// `bin::monitor::top`'s screen test gives the same readings. The bars
// are the page's own (D82 in `client/Spec.lean`).

import { describe, expect, test } from "bun:test";

import type { Sample } from "../wire";
import { FULL, plot, rows } from "./monitor";

const GIB = 1024 ** 3;

const ZERO: Sample = {
  core_cpu_permille: 0,
  core_private_bytes: 0,
  core_working_set_bytes: 0,
  core_read_bytes: 0,
  core_written_bytes: 0,
  machine_cpu_permille: 0,
  machine_available_bytes: 0,
  volume_free_bytes: 0,
  ledger_queue_depth: 0,
  durable_lag: 0,
  relay_p50_nanos: 0,
  event_to_screen_p50_nanos: 0,
  queued_runs: 0,
  view_backlog: 0,
  read_nanos: 0,
};

describe("monitor", () => {
  test("bars scale the window from its minimum to its maximum, with a p50 and p99 band", () => {
    expect([plot([0, 5, 10, 10_000]), plot([7, 7, 7]), plot([])]).toEqual([
      { bars: [80, 80, 80, FULL], p50: 80, p99: FULL },
      { bars: [500, 500, 500], p50: 500, p99: 500 },
      { bars: [], p50: 0, p99: 0 },
    ]);
  });

  test("a row per counter, its latest reading in its own unit", () => {
    const before: Sample = {
      core_cpu_permille: 100,
      core_private_bytes: 3 * GIB,
      core_working_set_bytes: 512,
      core_read_bytes: 0,
      core_written_bytes: 1536,
      machine_cpu_permille: 999,
      machine_available_bytes: 8 * GIB,
      volume_free_bytes: 2 ** 40,
      ledger_queue_depth: 0,
      durable_lag: 2,
      relay_p50_nanos: 900,
      event_to_screen_p50_nanos: 16_600_000,
      queued_runs: 1,
      view_backlog: 0,
      read_nanos: 0,
    };
    const after: Sample = {
      ...before,
      core_cpu_permille: 123,
      core_private_bytes: 3_382_286_746,
      durable_lag: 5,
      relay_p50_nanos: 1_500_000,
      queued_runs: 4,
    };
    const read = (width: number) =>
      rows([before, after], width).map(({ label, reading }) => ({ label, reading }));
    expect([read(60), rows([], 60)]).toEqual([
      [
        { label: "monitor_core_cpu", reading: "12.3%" },
        { label: "monitor_core_private", reading: "3.1 GiB" },
        { label: "monitor_core_working_set", reading: "512 B" },
        { label: "monitor_core_read", reading: "0 B" },
        { label: "monitor_core_written", reading: "1.5 KiB" },
        { label: "monitor_machine_cpu", reading: "99.9%" },
        { label: "monitor_machine_available", reading: "8.0 GiB" },
        { label: "monitor_volume_free", reading: "1.0 TiB" },
        { label: "monitor_ledger_queue_depth", reading: "0" },
        { label: "monitor_durable_lag", reading: "5" },
        { label: "monitor_relay_p50", reading: "1500 µs" },
        { label: "monitor_event_to_screen_p50", reading: "16.6 ms" },
        { label: "monitor_queued_runs", reading: "4" },
      ],
      [],
    ]);
  });

  test("a row's band is its window's p50 and p99, and the window is the last `width` samples", () => {
    const at = (cpu: number): Sample => ({ ...ZERO, core_cpu_permille: cpu });
    const [first] = rows([at(990), at(100), at(200), at(300), at(400)], 4);
    expect(first).toEqual({
      label: "monitor_core_cpu",
      reading: "40.0%",
      p50: "20.0%",
      p99: "40.0%",
      plot: { bars: [80, 386, 693, FULL], p50: 386, p99: FULL },
    });
  });
});
