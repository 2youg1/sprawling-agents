<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts" module>
  // The record's one timeline, in the three ways a person reads it: the
  // ledger and the log together, the log alone (what `#/record/log`
  // opens on), and a city whose ledger page is empty. The records cross
  // midnight in UTC, so the day is written twice, once above each day's
  // first row.

  import type { Answer, EventKind, EventRecord, LogLevel, LogLine, Payload, Query } from "../../wire";
  import { Address, B3Hash, RunId, Seq, TimeMs } from "../../wire";

  const RUN = RunId.make("3f2a9c4e-0d1b-4e7a-9a55-1c2b3d4e5f60");
  const MIDNIGHT = Date.UTC(2026, 9, 2);

  function written(seq: number, ms: number, kind: EventKind, data: Payload, addr: string | null): EventRecord {
    return {
      run: RUN,
      seq: Seq.make(seq),
      kind,
      t: TimeMs.make(MIDNIGHT + ms),
      who: "hall/mayor",
      addr: addr === null ? null : Address.make(addr),
      prev: B3Hash.make("0".repeat(64)),
      v: 1,
      data,
    };
  }

  function logged(seq: number, ms: number, level: LogLevel, module: string, line: string): LogLine {
    return { seq: Seq.make(seq), t: TimeMs.make(MIDNIGHT + ms), level, module, line, run: RUN };
  }

  const RECORDS: readonly EventRecord[] = [
    written(40, -2_412, "run_started", { task: "tidy the release notes" }, "shop/notes"),
    written(41, -1_870, "model_called", { model: "fable-large" }, "shop/notes"),
    written(42, -96, "tool_called", { tool: "read", subject: "CHANGELOG.md" }, "shop/notes"),
    written(43, 31, "tool_result", { tool: "read", outcome: "answered" }, "shop/notes"),
    written(44, 1_204, "checkpoint_committed", { files: 2 }, "shop/notes"),
    written(45, 1_760, "run_frozen", { completion: "done" }, "shop/notes"),
  ];

  const LOGS: readonly LogLine[] = [
    logged(41, -1_802, "effect", "gateway", "POST /v1/responses 200 in 1.84s"),
    logged(43, 38, "decide", "runtime", "the read stays under the window; no offload"),
    logged(44, 1_190, "effect", "memory", "commit 9c41e07 on shop/notes"),
    logged(45, 1_771, "trace", "runtime", "run frozen with completion done"),
  ];

  function history(records: readonly EventRecord[]): (query: Query) => Answer | undefined {
    return (query) => (typeof query === "object" && "history" in query ? { history: { records, earlier: null } } : undefined);
  }
</script>

<script lang="ts">
  import Timeline from "../record/timeline.svelte";
  import type { Source } from "../record/timeline";
  import Case from "./case.svelte";
  import Stand from "./stand.svelte";

  interface Shown {
    readonly name: string;
    readonly source: Source;
    readonly records: readonly EventRecord[];
    readonly width: number;
  }

  const SHOWN: readonly Shown[] = [
    { name: "ledger and log across midnight", source: "every", records: RECORDS, width: 1440 },
    { name: "the log alone, as #/record/log opens it", source: "log", records: RECORDS, width: 1440 },
    { name: "ledger and log at a phone's width", source: "every", records: RECORDS, width: 390 },
    { name: "an empty ledger page", source: "ledger", records: [], width: 1440 },
  ];
</script>

{#each SHOWN as shown (shown.name)}
  <Case label={`record · ${shown.name}`} width={shown.width}>
    <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={history(shown.records)} logs={LOGS}>
      <Timeline source={shown.source} onSource={() => undefined} />
    </Stand>
  </Case>
{/each}
