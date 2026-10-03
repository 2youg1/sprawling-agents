<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // A synchronous `send` outside the thread (client/Spec.lean D88): the
  // mailbox's working rows and the runs board while a planner waits for
  // a parser room to answer, and after each of the three ways the wait
  // ends. The mailbox reads its rows from records folded through the
  // belief's own door, so the posture each row shows is the one the
  // fold derives; the board is handed the same postures as runs.
  //
  // After a reply or a timeout the run is thinking again; a run that
  // left its room is at work with no phase until its `run_frozen`
  // arrives. How the wait ended is the thread's note to say (`Note::
  // AwaitingReply`), not these rows'.

  import { afterWait } from "../../core/doing";
  import type { Doing } from "../../core/doing";
  import type { BoardRun } from "../runs/lineage";
  import type { EventKind, EventRecord } from "../../wire";
  import { Address, B3Hash, RunId, Seq, TimeMs } from "../../wire";

  // The fixture's clock: the mailbox counts on the page's own clock, so
  // its waits are placed against it as the mailbox fixture's runs are.
  const NOW = Date.now();
  const MINUTE = 60_000;
  const PLANNER = Address.make("lab/planner");
  const PARSER = Address.make("lab/parser");
  const RUN = RunId.make("0199c0de-a9e0-4000-8000-000000000011");
  const TASK = "Split the parser work and wait for the parser room to confirm the error positions.";
  const UNTIL = TimeMs.make(NOW + 4 * MINUTE);

  type Ending = "reply" | "timeout" | "left";

  function record(seq: number, ago: number, kind: EventKind, data: Record<string, unknown>): EventRecord {
    return {
      addr: PLANNER,
      data,
      kind,
      prev: B3Hash.make("0".repeat(64)),
      run: RUN,
      seq: Seq.make(seq),
      t: TimeMs.make(NOW - ago),
      v: 1,
      who: PLANNER,
    };
  }

  // The planner's run up to the wait, and the line that ends it when
  // the case shows an ending.
  function recordsOf(ending: Ending | null): readonly EventRecord[] {
    const waiting = [
      record(1, 6 * MINUTE, "run_started", { task: TASK }),
      record(2, 2 * MINUTE, "tool_called", { name: "signal", subject: PARSER }),
      record(3, 2 * MINUTE, "signal_wait_started", { on: PARSER, signal: `${RUN}-s1`, deadline_ms: UNTIL }),
    ];
    if (ending === null) return waiting;
    const by = ending === "reply" ? { end: "reply", reply: "parser-s1" } : { end: ending };
    return [...waiting, record(4, 1 * MINUTE, "signal_wait_ended", { signal: `${RUN}-s1`, by })];
  }

  const AWAITING: Doing = { kind: "awaiting_reply", wait: { on: PARSER, until: UNTIL } };

  function boardOf(doing: Doing): readonly BoardRun[] {
    return [{ run: RUN, addr: PLANNER, task: TASK, goal: null, started: NOW - 6 * MINUTE, ended: null, doing }];
  }

  const CASES: readonly { readonly label: string; readonly records: readonly EventRecord[]; readonly board: readonly BoardRun[] }[] = [
    {
      label: "a sync wait in progress",
      records: recordsOf(null),
      board: boardOf(AWAITING),
    },
    { label: "the wait ended by a reply", records: recordsOf("reply"), board: boardOf(afterWait(AWAITING, "reply")) },
    { label: "the wait ended by its timeout", records: recordsOf("timeout"), board: boardOf(afterWait(AWAITING, "timeout")) },
    { label: "the wait ended by the run leaving", records: recordsOf("left"), board: boardOf(afterWait(AWAITING, "left")) },
  ];
</script>

<script lang="ts">
  import Column from "../mailbox/column.svelte";
  import Board from "../runs/board.svelte";
  import Case from "./case.svelte";
  import Stand from "./stand.svelte";

  const LIVE = { kind: "live", city: "sprawling" } as const;
  const ignore = (): void => undefined;
</script>

{#each CASES as shown (shown.label)}
  <Case label={`agent messages · ${shown.label}, in the mailbox`} width={440}>
    <Stand link={LIVE} unread={[]} waiting={[]} records={shown.records}>
      <div class="flex h-[360px] flex-col bg-raised">
        <Column onClose={ignore} />
      </div>
    </Stand>
  </Case>
  <Case label={`agent messages · ${shown.label}, on the runs board`} width={1200}>
    <Board runs={shown.board} now={NOW} level={2} />
  </Case>
{/each}
