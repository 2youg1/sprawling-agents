<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // What residents say to each other, as today's wire lets a page draw
  // it (client D83): a planner the User asked for work hands part of it
  // down to a parser room and sends it a signal without waiting; the
  // child room opens on the planner's words, named and linked rather
  // than signed "you", pulls the signals waiting for it, and still has
  // two more queued. The sessions pane's rows of the two, and the
  // openings the User and the city dispatched, are the cases of
  // `agent_children.svelte`.
  //
  // The send and the pull are drawn the way the generic tool line draws
  // them; a received signal as a letter, a reply wait and a handback are
  // the cases of `agent_letters.svelte` (client D86).

  import type { Answer, Call, EventKind, EventRecord, Query, SignalLine, Turn } from "../../wire";
  import { Address, B3Hash, RunId, Seq, TimeMs, Tokens } from "../../wire";

  export const PARENT = Address.make("lab/planner");
  export const CHILD = Address.make("lab/parser");
  const MODEL = "anthropic/claude-sonnet-5";
  const PARENT_RUN = RunId.make("0199c0de-a9e0-4000-8000-000000000001");
  const CHILD_RUN = RunId.make("0199c0de-a9e0-4000-8000-000000000002");
  const PARENT_TASK = "Split the parser work: keep the grammar here, hand the error positions to a room of their own.";
  const CHILD_TASK = "Report every parse error as a byte offset from the start of the input, and say when the tests pass.";
  const CHILD_GOAL = "The parser's errors carry byte offsets and its tests pass.";
  const SENT = "The grammar keeps `Span` as two byte offsets; build the error positions on it.";

  // Each run: its id, its room, the line it started on, its task and
  // who dispatched it, as `run_started` records them.
  const RUNS: readonly (readonly [RunId, Address, number, string, string])[] = [
    [PARENT_RUN, PARENT, 10, PARENT_TASK, "person"],
    [CHILD_RUN, CHILD, 20, CHILD_TASK, PARENT],
  ];

  function record(run: RunId, room: Address, seq: number, at: number, kind: EventKind, data: Record<string, unknown>): EventRecord {
    return { run, seq: Seq.make(seq), kind, t: TimeMs.make(at), who: "city", addr: room, prev: B3Hash.make("0".repeat(64)), v: 1, data };
  }

  function startOf(seq: number, now: number): number {
    return now - (40 - seq) * 30_000;
  }

  export function recordsAt(now: number): readonly EventRecord[] {
    return RUNS.flatMap(([run, room, seq, task, by]) => [
      record(run, room, seq, startOf(seq, now), "run_started", { task, dispatched_by: by }),
      record(run, room, seq + 1, startOf(seq, now) + 1_000, "model_called", { model: MODEL }),
      record(run, room, seq + 2, startOf(seq, now) + 60_000, "run_frozen", { completion: "done" }),
    ]);
  }

  // A signal call as the generic line draws it: the tool, the action the
  // city reads as its subject, and what it answered.
  function signal(at: number, called: number, action: string, answered: string): Call {
    return {
      tool: "signal",
      subject: action,
      arguments: null,
      outcome: "answered",
      at: Seq.make(at),
      output: { head: answered, cut: 0 },
      called: TimeMs.make(called),
      answered: TimeMs.make(called + 40),
      timing: "measured",
      effect: { write: { domain: Address.make("lab") } },
      render: "generic",
      took_us: 40_000,
    };
  }

  function turn(seq: number, t: number, said: string, calls: readonly Call[]): Turn {
    return {
      calls,
      notes: [],
      number: 1,
      opened: Seq.make(seq * 10),
      t: TimeMs.make(t),
      timing: "measured",
      first_at: TimeMs.make(t + 450),
      model: MODEL,
      said,
      used: { input: Tokens.make(18_000), output: Tokens.make(700), cached: Tokens.make(9_000) },
      stopped: "end_turn",
    };
  }

  function rounds(run: RunId, now: number): Answer | undefined {
    const found = RUNS.find(([id]) => id === run);
    if (found === undefined) return undefined;
    const [, , seq, task, by] = found;
    const t = startOf(seq, now) + 1_000;
    const calls =
      run === PARENT_RUN
        ? [signal(seq * 10 + 1, t + 2_000, "send", JSON.stringify({ sent: true, to: CHILD }))]
        : [signal(seq * 10 + 1, t + 2_000, "pull", JSON.stringify([{ from: PARENT, kind: "thread", text: SENT, sender: "running" }]))];
    const said =
      run === PARENT_RUN
        ? "The error positions are with lab/parser now; I told it which span type to build on and kept the grammar."
        : "Error positions are byte offsets on the grammar's `Span`, and the parser tests pass.";
    return {
      rounds: {
        run,
        turns: [turn(seq, t, said, calls)],
        opened_at: null,
        opening: {
          at: TimeMs.make(startOf(seq, now)),
          task,
          goal: run === CHILD_RUN ? CHILD_GOAL : "",
          dispatched_by: by,
          parent: run === CHILD_RUN ? PARENT_RUN : null,
          policy: { mode: "work", write: "full", admit: "standing", landing: "ordinary" },
        },
        worktree: null,
      },
    };
  }

  // The signals still queued for the child after its run pulled the first.
  function queued(now: number): readonly SignalLine[] {
    return [
      { id: "sig-2", from: PARENT, kind: "thread", at: TimeMs.make(now - 120_000) },
      { id: "sig-3", from: "hall/mayor", kind: "mention", at: TimeMs.make(now - 40_000) },
    ];
  }

  export function answering(now: number): (query: Query) => Answer | undefined {
    return (query) => {
      if (query === "preferences") return { preferences: { tags: [] } };
      if (typeof query !== "object") return undefined;
      if ("rounds" in query) return rounds(query.rounds.run, now);
      if ("inbox_view" in query) {
        return { inbox: { addr: query.inbox_view.addr, waiting: query.inbox_view.addr === CHILD ? queued(now) : [] } };
      }
      return undefined;
    };
  }
</script>

<script lang="ts">
  import { ui } from "../../ui";
  import Talk from "../talk.svelte";
  import Case from "./case.svelte";
  import Stand from "./stand.svelte";

  const u = ui();
  const now = u.now();
  const records = recordsAt(now);
  const answers = answering(now);
</script>

<Case label="agent messages · a child room opened by a resident, a pulled signal, two signals queued" width={760}>
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} {answers} {records}>
    <div class="h-[640px] px-wide">
      <Talk address={CHILD} band={false} />
    </div>
  </Stand>
</Case>

<Case label="agent messages · the parent's thread, an async send as drawn today" width={760}>
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} {answers} {records}>
    <div class="h-[520px] px-wide">
      <Talk address={PARENT} band={false} />
    </div>
  </Stand>
</Case>

