// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";
import { get } from "svelte/store";

import { createWatching } from "./watching";

describe("watching the monitor", () => {
  test("the city hears one watch for many panels, and a release only when the last closes", () => {
    const sent: string[] = [];
    const watching = createWatching((text) => {
      sent.push(text);
      return true;
    });
    const first = watching.watch();
    const second = watching.watch();
    watching.reconnected();
    first();
    first();
    second();
    watching.reconnected();
    expect(sent).toEqual([
      '{"monitor":"watch"}',
      '{"monitor":"watch"}',
      '{"monitor":"release"}',
    ]);
  });

  test("a page keeps the last 300 readings and forgets them on release", () => {
    const watching = createWatching(() => true);
    const release = watching.watch();
    const zero = {
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
    };
    for (let second = 0; second < 301; second += 1) {
      watching.sampled({ ...zero, queued_runs: second });
    }
    const kept = get(watching.samples);
    expect([kept.length, kept[0]?.queued_runs, kept[299]?.queued_runs]).toEqual([300, 1, 300]);
    release();
    expect(get(watching.samples)).toEqual([]);
  });
});
