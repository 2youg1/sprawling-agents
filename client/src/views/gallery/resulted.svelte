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
  // Each run began a set number of minutes before the stand's clock, so
  // the rows fall into all three recency bands on every day it is shown,
  // and every finished run answers the same rounds and changes, so its
  // row carries a produced line.
  import type { Answer, EventKind, EventRecord, Query } from "../../wire";
  import { Address, B3Hash, GitOid, RunId, Seq, TimeMs } from "../../wire";

  const ROOMS = ["release/ledger", "memory/gate", "memory/tidy", "docs/tidy", "runtime/fix", "shop/gate"];

  function record(index: number, at: number, kind: EventKind, data: Record<string, unknown>): EventRecord {
    return {
      run: RunId.make(`0199c0de-0000-4000-8000-${index.toString(16).padStart(12, "0")}`),
      seq: Seq.make(index * 3 + (kind === "run_started" ? 1 : kind === "pr_opened" ? 2 : 3)),
      kind,
      t: TimeMs.make(at),
      who: "city",
      addr: Address.make(ROOMS[index % ROOMS.length] ?? "hall/mayor"),
      prev: B3Hash.make("0".repeat(64)),
      v: 1,
      data,
    };
  }

  // Each run starts some minutes ago; then it asks, stops short,
  // finishes or keeps going.
  const ENDINGS: readonly (readonly [number, EventKind, Record<string, unknown>] | null)[] = [
    [1, "approval_requested", { action_desc: "run cargo publish --dry-run outside the sandbox" }],
    [52, "approval_requested", { action_desc: "send a request to api.github.com" }],
    [3, "run_frozen", { completion: "limit" }],
    [70, "run_frozen", { completion: null }],
    ...[2, 8, 14, 33, 47, 95, 130, 180].map((ago): readonly [number, EventKind, Record<string, unknown>] => [
      ago,
      "run_frozen",
      { completion: "done" },
    ]),
    null,
  ];

  // The finished runs that opened a pull request, by their index above.
  const PULLED: ReadonlyMap<number, string> = new Map([
    [4, "ledger-composer"],
    [7, "glossary-dedup"],
  ]);

  export function recordsAt(now: number): readonly EventRecord[] {
    return ENDINGS.flatMap((ending, index) => {
      const began = now - (ending?.[0] ?? 0) * 60_000;
      const opened = record(index, began, "run_started", { task: `task ${String(index)}` });
      if (ending === null) return [opened];
      const ended = record(index, began + 30_000, ending[1], ending[2]);
      const pr = PULLED.get(index);
      return pr === undefined
        ? [opened, ended]
        : [opened, record(index, began + 20_000, "pr_opened", { branch: pr }), ended];
    });
  }

  // The tree every finished run opened at, and what it changed since.
  const OPENED: GitOid = GitOid.make("3f2a9c1e7b4d5a6f8e0c1b2d3a4f5e6d7c8b9a01");

  export function finished(query: Query): Answer | undefined {
    if (typeof query !== "object") return undefined;
    if ("rounds" in query) return { rounds: { run: query.rounds.run, turns: [], opened_at: OPENED } };
    if (!("changes" in query)) return undefined;
    return {
      changes: {
        base: OPENED,
        head: null,
        files: [
          { path: "crates/kernel/src/ledger.rs", how: "modified", lines: { counted: { added: 90, removed: 36 } } },
          { path: "crates/kernel/src/ledger/tests.rs", how: "added", lines: { counted: { added: 37, removed: 0 } } },
        ],
      },
    };
  }
</script>

<script lang="ts">
  import Results from "../city/results.svelte";
  import { ui } from "../../ui";
  import Case from "./case.svelte";
  import Stand from "./stand.svelte";

  const records = recordsAt(ui().now());
</script>

<Case label="city · results only, waiting, failed and just finished">
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={finished} {records}>
    <Results />
  </Stand>
</Case>
