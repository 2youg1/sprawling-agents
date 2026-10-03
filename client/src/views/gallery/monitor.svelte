<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The performance panel before its first sample, with ten seconds
  // behind it whose readings run from a few nanoseconds to terabytes,
  // and with a full five minutes behind it. The history is a fixed script rather than a
  // random one, so the render gate sees the same curves every time.

  import type { Sample } from "../../wire";

  const GIB = 1024 ** 3;

  // Three hundred samples, one a second, each counter on its own
  // saw-tooth so every curve reaches its lowest and highest block.
  const FULL: readonly Sample[] = Array.from({ length: 300 }, (_, second) => ({
    core_cpu_permille: 20 + (second % 40),
    core_private_bytes: 180 * 1024 * 1024 + (second % 60) * 65_536,
    core_working_set_bytes: 220 * 1024 * 1024 + (second % 30) * 131_072,
    core_read_bytes: (second % 15) * 4096,
    core_written_bytes: (second % 20) * 8192,
    machine_cpu_permille: 150 + (second % 90) * 3,
    machine_available_bytes: 9 * GIB + (second % 50) * 1_048_576,
    volume_free_bytes: 400 * GIB - second * 65_536,
    ledger_queue_depth: second % 5,
    durable_lag: second % 3,
    relay_p50_nanos: 400_000 + (second % 25) * 20_000,
    event_to_screen_p50_nanos: 8_000_000 + (second % 45) * 100_000,
    queued_runs: Math.floor(second / 60),
    view_backlog: second % 7,
    read_nanos: 1_000 + (second % 9) * 100,
    beat_ms: 100,
  }));

  // Ten seconds where each counter moves by a sliver of a large value or
  // by one unit of a small one, the two cases a plot scaled from zero
  // draws flat: the window's own scale has to show both.
  const SHORT: readonly Sample[] = Array.from({ length: 10 }, (_, second) => ({
    core_cpu_permille: 1 + (second % 2),
    core_private_bytes: 3_330_000_000 + second * 10_000_000,
    core_working_set_bytes: 3_400_000_000 - (second % 4) * 25_000_000,
    core_read_bytes: second === 6 ? 512 : 0,
    core_written_bytes: 2 ** 40 + second,
    machine_cpu_permille: 997 + (second % 3),
    machine_available_bytes: 64 * GIB - second * 4096,
    volume_free_bytes: 900 * GIB,
    ledger_queue_depth: second % 2,
    durable_lag: second === 9 ? 40 : 0,
    relay_p50_nanos: 700 + second * 40,
    event_to_screen_p50_nanos: 9_990_000 + second * 5_000,
    queued_runs: 0,
    view_backlog: 0,
    read_nanos: 1_000,
    beat_ms: 100,
  }));

  // The gallery has no city to ask, so opening the panel starts nothing.
  function watch(): () => void {
    return () => undefined;
  }
</script>

<script lang="ts">
  import Monitor from "../monitor.svelte";
  import Case from "./case.svelte";
</script>

<Case label="monitor · before the first sample">
  <Monitor samples={[]} {watch} rank="section" />
</Case>

<Case label="monitor · ten seconds, small and large readings">
  <Monitor samples={SHORT} {watch} rank="section" />
</Case>

<Case label="monitor · five minutes of samples">
  <Monitor samples={FULL} {watch} rank="section" />
</Case>
