<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The city in results-only mode: two runs wait for the person, two
  // failed, eight finished and one is still working. The finished group
  // is longer than `FIRST`, so the fixture shows both the cut and the
  // count of what was cut; the working run is in none of the groups.
  import type { EventKind, EventRecord } from "../../wire";
  import { B3Hash, RunId, Seq, TimeMs } from "../../wire";

  // One minute apart from a fixed start, so the rows' ages are stable.
  const START = 1_767_225_600_000;

  function record(index: number, at: number, kind: EventKind, data: Record<string, unknown>): EventRecord {
    return {
      run: RunId.make(`0199c0de-0000-4000-8000-${index.toString(16).padStart(12, "0")}`),
      seq: Seq.make(at),
      kind,
      t: TimeMs.make(START + at * 60_000),
      who: `hall/resident-${String(index)}`,
      prev: B3Hash.make("0".repeat(64)),
      v: 1,
      data,
    };
  }

  // Each run starts; then it asks, stops short, finishes or keeps going.
  const ENDINGS: readonly (readonly [EventKind, Record<string, unknown>] | null)[] = [
    ["approval_requested", {}],
    ["approval_requested", {}],
    ["run_frozen", { completion: "limit" }],
    ["run_frozen", { completion: "failed" }],
    ...Array.from({ length: 8 }, (): readonly [EventKind, Record<string, unknown>] => [
      "run_frozen",
      { completion: "done" },
    ]),
    null,
  ];

  export const RECORDS: readonly EventRecord[] = ENDINGS.flatMap((ending, index) => {
    const opened = record(index, index * 2 + 1, "run_started", { task: `task ${String(index)}` });
    return ending === null ? [opened] : [opened, record(index, index * 2 + 2, ending[0], ending[1])];
  });
</script>

<script lang="ts">
  import Results from "../city/results.svelte";
  import Case from "./case.svelte";
  import Stand from "./stand.svelte";
</script>

<Case label="city · results only, waiting, failed and just finished">
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} records={RECORDS}>
    <Results />
  </Stand>
</Case>
