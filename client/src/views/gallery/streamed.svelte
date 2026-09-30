<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The room in results-only mode: a session forked from an earlier run,
  // holding a run still working, one that failed without committing and
  // one that finished and committed; and an earlier session with one run
  // that waits for the person. Every run's rounds end on what it said,
  // and the two that fenced answer the same changes.
  import type { RunBelief } from "../../core/belief";
  import type { Doing } from "../../core/doing";
  import type { Answer, Query, Turn } from "../../wire";
  import { Address, GitOid, RunId, Seq, TimeMs } from "../../wire";

  const ROOM = Address.make("kernel-lab/gate");
  const OPENED: GitOid = GitOid.make("3f2a9c1e7b4d5a6f8e0c1b2d3a4f5e6d7c8b9a01");
  const FENCED: GitOid = GitOid.make("9b8c7d6e5f4a3b2c1d0e9f8a7b6c5d4e3f2a1b0c");

  interface Told {
    readonly task: string;
    readonly ago: number;
    readonly doing: Doing;
    readonly said: string;
    readonly fenced: boolean;
    readonly ask: string | null;
  }

  const TOLD: readonly Told[] = [
    {
      task: "Make the gate's dedup an index instead of a linear scan",
      ago: 4,
      doing: { kind: "thinking" },
      said: "",
      fenced: false,
      ask: null,
    },
    {
      task: "Run the whole clippy pass and fix the warnings",
      ago: 156,
      doing: { kind: "frozen", completion: "limit" },
      said: "Fixed 11 of the 14 warnings; the other 3 are in runtime, outside this room's write set.",
      fenced: false,
      ask: null,
    },
    {
      task: "Switch the index to a BTreeMap so every machine orders it the same",
      ago: 205,
      doing: { kind: "frozen", completion: "done" },
      said: "ClusterIndex is now a BTreeMap; the kernel decision path no longer depends on hash order. The seen and release tests are unchanged and green.",
      fenced: true,
      ask: null,
    },
    {
      task: "Publish the gate crate",
      ago: 260,
      doing: { kind: "waiting" },
      said: "The crate is ready; publishing needs the network outside the sandbox.",
      fenced: false,
      ask: "run cargo publish --dry-run outside the sandbox",
    },
  ];

  function runOf(index: number): RunId {
    return RunId.make(`0199c0de-0000-4000-9000-${index.toString(16).padStart(12, "0")}`);
  }

  export function beliefsAt(now: number): readonly RunBelief[] {
    return TOLD.map((told, index) => ({
      run: runOf(index),
      addr: ROOM,
      started: TimeMs.make(now - told.ago * 60_000),
      task: told.task,
      goal: null,
      lastSeq: Seq.make(100 - index),
      doing: told.doing,
      model: null,
      pr: told.fenced ? "gate-btree" : null,
      ask: told.ask,
      local: false,
      saying: "",
      thinking: "",
    }));
  }

  function turnsOf(told: Told): readonly Turn[] {
    return [
      {
        calls: [],
        notes: told.fenced ? [{ checkpointed: { at: Seq.make(9), oid: FENCED } }] : [],
        number: 1,
        opened: Seq.make(2),
        said: told.said,
        t: TimeMs.make(0),
        timing: "measured",
      },
    ];
  }

  export function streamed(query: Query): Answer | undefined {
    if (typeof query !== "object") return undefined;
    if ("rounds" in query) {
      const run = query.rounds.run;
      const told = TOLD[TOLD.findIndex((_, index) => runOf(index) === run)];
      return told === undefined
        ? undefined
        : { rounds: { run, turns: turnsOf(told), opened_at: OPENED } };
    }
    if (!("changes" in query)) return undefined;
    return {
      changes: {
        base: OPENED,
        head: FENCED,
        files: [
          { path: "crates/kernel/src/gate/cluster.rs", how: "modified", lines: { counted: { added: 8, removed: 11 } } },
          { path: "crates/kernel/src/gate/seen.rs", how: "modified", lines: { counted: { added: 2, removed: 0 } } },
        ],
      },
    };
  }
</script>

<script lang="ts">
  import Stream from "../talk/stream.svelte";
  import { ui } from "../../ui";
  import Case from "./case.svelte";
  import Stand from "./stand.svelte";

  const runs = beliefsAt(ui().now());
</script>

<Case label="room · results only, by session, one block per run">
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={streamed}>
    <Stream
      shown={runs.slice(0, 3).reverse()}
      earlier={runs.slice(3)}
      boundary={{ kind: "forked", at: null, turn: 4, mother: runOf(9) }}
    />
  </Stand>
</Case>
