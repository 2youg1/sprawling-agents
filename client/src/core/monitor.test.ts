// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The panel draws what `sprawling top` draws: the same fixture as
// `bin::monitor::top`'s screen test gives the same rows.

import { describe, expect, test } from "bun:test";

import type { Sample } from "../wire";
import { rows, sparkline } from "./monitor";

const GIB = 1024 ** 3;

describe("monitor", () => {
  test("a curve scales the latest window from its minimum to its maximum", () => {
    expect([
      sparkline([0, 1, 2, 3, 4, 5, 6, 7], 8),
      sparkline([100, 0, 7, 14], 2),
      sparkline([5, 5, 5], 10),
      sparkline([], 4),
    ]).toEqual(["▁▂▃▄▅▆▇█", "▁█", "▁▁▁", ""]);
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
    };
    const after: Sample = {
      ...before,
      core_cpu_permille: 123,
      core_private_bytes: 3_382_286_746,
      durable_lag: 5,
      relay_p50_nanos: 1_500_000,
      queued_runs: 4,
    };
    expect([rows([before, after], 60), rows([], 60)]).toEqual([
      [
        { label: "monitor_core_cpu", reading: "12.3%", curve: "▁█" },
        { label: "monitor_core_private", reading: "3.1 GiB", curve: "▁█" },
        { label: "monitor_core_working_set", reading: "512 B", curve: "▁▁" },
        { label: "monitor_core_read", reading: "0 B", curve: "▁▁" },
        { label: "monitor_core_written", reading: "1.5 KiB", curve: "▁▁" },
        { label: "monitor_machine_cpu", reading: "99.9%", curve: "▁▁" },
        { label: "monitor_machine_available", reading: "8.0 GiB", curve: "▁▁" },
        { label: "monitor_volume_free", reading: "1.0 TiB", curve: "▁▁" },
        { label: "monitor_ledger_queue_depth", reading: "0", curve: "▁▁" },
        { label: "monitor_durable_lag", reading: "5", curve: "▁█" },
        { label: "monitor_relay_p50", reading: "1.5 ms", curve: "▁█" },
        { label: "monitor_event_to_screen_p50", reading: "16.6 ms", curve: "▁▁" },
        { label: "monitor_queued_runs", reading: "4", curve: "▁█" },
      ],
      [],
    ]);
  });
});
