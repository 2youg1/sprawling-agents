<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The agent messages the thread draws from the notes the wire now
  // carries (client D86): the User's steer beside a resident's letter,
  // a reply wait in progress and each of its three endings, a child's
  // handback finished and stopped, and two queued signals with their
  // first lines. A letter names its kind and links the session that sent
  // it, and one from an older Ledger names neither (client D90); four
  // sends say where each letter landed - delivered, queued, knocked - and
  // what the tool answered when the Ledger recorded no landing (wire
  // D42). Each case is one room with one run, so each thread shows only
  // its own case.

  import type { Answer, Call, EventKind, EventRecord, Landing, Note, Query, SignalLine, Turn } from "../../wire";
  import { Address, B3Hash, RunId, Seq, TimeMs, Tokens } from "../../wire";

  const MODEL = "anthropic/claude-sonnet-5";
  const PLANNER = Address.make("lab/planner");
  const PARSER = Address.make("lab/parser");

  export const LETTERS = Address.make("lab/parser");
  export const WAITING = Address.make("lab/planner");
  export const REPLIED = Address.make("lab/review");
  export const TIMED_OUT = Address.make("lab/docs");
  export const LEFT = Address.make("lab/ops");
  export const FINISHED = Address.make("lab/lead");
  export const STOPPED = Address.make("lab/chief");
  export const QUEUED = Address.make("lab/queue");
  export const OLDER = Address.make("lab/archive");
  export const SENDS = Address.make("lab/sends");

  const CHILD_SESSION = RunId.make("0199c0de-b0b0-4000-8000-0000000000c1");
  const PLANNER_SESSION = RunId.make("0199c0de-b0b0-4000-8000-0000000000c2");
  const PARSER_SESSION = RunId.make("0199c0de-b0b0-4000-8000-0000000000c3");

  // Who sent a letter, as the paired sending line says (wire D43).
  interface Sent {
    readonly kind: string;
    readonly session: RunId;
  }

  interface Scene {
    readonly room: Address;
    readonly task: string;
    readonly said: string;
    readonly running: boolean;
    readonly notes: (seq: number, t: number, now: number) => readonly Note[];
    readonly calls: (seq: number, t: number) => readonly Call[];
  }

  function arrived(at: number, t: number, by: "user" | "resident", from: string, said: string, sent: Sent | null = null): Note {
    return {
      arrived: { at: Seq.make(at), by, from, said, t: TimeMs.make(t), handback: null, kind: sent?.kind ?? null, session: sent?.session ?? null },
    };
  }

  function awaiting(at: number, t: number, on: Address, until: number, ended: Note | null): readonly Note[] {
    const end = ended === null || !("arrived" in ended) ? null : ended.arrived;
    return [
      {
        awaiting_reply: {
          at: Seq.make(at),
          on,
          t: TimeMs.make(t),
          until: TimeMs.make(until),
          ended: end === null ? null : { by: "reply", t: end.t },
        },
      },
      ...(ended === null ? [] : [ended]),
    ];
  }

  function ending(at: number, t: number, on: Address, until: number, by: "timeout" | "left", endedAt: number): Note {
    return { awaiting_reply: { at: Seq.make(at), on, t: TimeMs.make(t), until: TimeMs.make(until), ended: { by, t: TimeMs.make(endedAt) } } };
  }

  function handback(at: number, t: number, from: Address, outcome: "finished" | "stopped"): Note {
    return {
      arrived: {
        at: Seq.make(at),
        by: "resident",
        from,
        said: null,
        t: TimeMs.make(t),
        kind: "thread",
        session: CHILD_SESSION,
        handback:
          outcome === "finished"
            ? { finished: { verified_by: "cargo nextest run -p parser" } }
            : { stopped: { because: "the grammar's Span type changed under it; the offsets need a decision" } },
      },
    };
  }

  // A sent letter as the tool line draws a `signal` call, with where it
  // landed when the Ledger recorded it (wire D42).
  function sent(at: number, called: number, to: Address, text: string, landing: Landing | null): Call {
    return {
      tool: "signal",
      subject: "send",
      arguments: { head: JSON.stringify({ action: "send", to, text }), cut: 0 },
      outcome: "answered",
      at: Seq.make(at),
      output: { head: JSON.stringify({ id: `sends-s${String(at)}`, to, kind: "mention", delivered: true }), cut: 0 },
      called: TimeMs.make(called),
      answered: TimeMs.make(called + 12),
      timing: "measured",
      effect: { write: { domain: Address.make("lab") } },
      render: "signal",
      took_us: 12_000,
      landing,
    };
  }

  // A pulled signal, drawn the way the tool line draws a `signal` call.
  function pulled(at: number, called: number): Call {
    return {
      tool: "signal",
      subject: "pull",
      arguments: { head: JSON.stringify({ action: "pull" }), cut: 0 },
      outcome: "answered",
      at: Seq.make(at),
      output: { head: JSON.stringify([{ from: PLANNER, kind: "thread", text: "Build the error positions on `Span`.", sender: "running" }]), cut: 0 },
      called: TimeMs.make(called),
      answered: TimeMs.make(called + 40),
      timing: "measured",
      effect: { write: { domain: Address.make("lab") } },
      render: "generic",
      took_us: 40_000,
    };
  }

  const CASES: readonly Scene[] = [
    {
      room: LETTERS,
      task: "Report every parse error as a byte offset from the start of the input.",
      said: "Offsets are on `Span` now; the planner's letter and your note agree, so I kept the grammar as it is.",
      running: false,
      notes: (seq, t) => [
        arrived(seq + 3, t + 4_000, "user", "user", "Keep the grammar file as it is; only the errors change."),
        arrived(
          seq + 4,
          t + 6_000,
          "resident",
          PLANNER,
          "The grammar keeps `Span` as two byte offsets.\nBuild the error positions on it, and tell me when the tests pass.",
          { kind: "thread", session: PLANNER_SESSION },
        ),
      ],
      calls: (seq, t) => [pulled(seq + 5, t + 7_000)],
    },
    {
      room: WAITING,
      task: "Agree the error format with the parser room before the release notes are written.",
      said: "I asked lab/parser which offset base it uses and I am waiting for its answer.",
      running: true,
      notes: (seq, t, now) => awaiting(seq + 3, t + 3_000, PARSER, now + 4 * 60_000, null),
      calls: () => [],
    },
    {
      room: REPLIED,
      task: "Check the parser's error offsets against the review checklist.",
      said: "lab/parser answered: offsets count bytes from zero, which matches the checklist.",
      running: false,
      notes: (seq, t) =>
        awaiting(
          seq + 3,
          t + 3_000,
          PARSER,
          t + 5 * 60_000,
          arrived(seq + 4, t + 45_000, "resident", PARSER, "Byte offsets, counted from zero.", { kind: "mention", session: PARSER_SESSION }),
        ),
      calls: () => [],
    },
    {
      room: TIMED_OUT,
      task: "Write the error-format section of the docs.",
      said: "lab/parser did not answer in time, so the section says the offset base is still to be confirmed.",
      running: false,
      notes: (seq, t) => [ending(seq + 3, t + 3_000, PARSER, t + 2 * 60_000, "timeout", t + 2 * 60_000)],
      calls: () => [],
    },
    {
      room: LEFT,
      task: "Ask the parser room whether the release build needs a new flag.",
      said: "lab/parser's run ended before it answered; I will ask again when it next runs.",
      running: false,
      notes: (seq, t) => [ending(seq + 3, t + 3_000, PARSER, t + 5 * 60_000, "left", t + 50_000)],
      calls: () => [],
    },
    {
      room: FINISHED,
      task: "Hand the error positions down to a room of their own and merge the result.",
      said: "lab/parser handed the work back finished and verified; the error positions are merged.",
      running: false,
      notes: (seq, t) => [handback(seq + 3, t + 8_000, PARSER, "finished")],
      calls: () => [],
    },
    {
      room: STOPPED,
      task: "Hand the error positions down to a room of their own and merge the result.",
      said: "lab/parser stopped and said why; the Span decision is mine to make before it can go on.",
      running: false,
      notes: (seq, t) => [handback(seq + 3, t + 8_000, PARSER, "stopped")],
      calls: () => [],
    },
    {
      room: OLDER,
      task: "Read what the planner sent before the city recorded a letter's kind.",
      said: "The planner's old letter asks for byte offsets; nothing in it says which session sent it.",
      running: false,
      notes: (seq, t) => [arrived(seq + 3, t + 4_000, "resident", PLANNER, "Use byte offsets for every error position.")],
      calls: () => [],
    },
    {
      room: SENDS,
      task: "Tell the three rooms that the error format is settled.",
      said: "Told all three: the parser read it at once, the docs room will read it on its next run, and review was woken for it.",
      running: false,
      notes: () => [],
      calls: (seq, t) => [
        sent(seq + 3, t + 2_000, PARSER, "The error format is settled: byte offsets from zero.", "delivered"),
        sent(seq + 4, t + 2_100, Address.make("lab/docs"), "Error positions are byte offsets now; the section can say so.", "queued"),
        sent(seq + 5, t + 2_200, Address.make("lab/review"), "Please check the new error offsets against the checklist.", "knocked"),
        sent(seq + 6, t + 2_300, PLANNER, "Settled, as you asked.", null),
      ],
    },
    {
      room: QUEUED,
      task: "Tidy the error messages once the parser work lands.",
      said: "Done with the messages; two signals arrived while I worked and wait for the next run.",
      running: false,
      notes: () => [],
      calls: () => [],
    },
  ];

  function runOf(index: number): RunId {
    return RunId.make(`0199c0de-b0b0-4000-8000-${String(index + 1).padStart(12, "0")}`);
  }

  function startOf(index: number, now: number): number {
    return now - (CASES.length - index) * 5 * 60_000;
  }

  function seqOf(index: number): number {
    return (index + 1) * 10;
  }

  function record(run: RunId, room: Address, seq: number, at: number, kind: EventKind, data: Record<string, unknown>): EventRecord {
    return { run, seq: Seq.make(seq), kind, t: TimeMs.make(at), who: "city", addr: room, prev: B3Hash.make("0".repeat(64)), v: 1, data };
  }

  export function recordsAt(now: number): readonly EventRecord[] {
    return CASES.flatMap((one, index) => {
      const run = runOf(index);
      const seq = seqOf(index);
      const at = startOf(index, now);
      return [
        record(run, one.room, seq, at, "run_started", { task: one.task, dispatched_by: "person" }),
        record(run, one.room, seq + 1, at + 1_000, "model_called", { model: MODEL }),
        ...(one.running ? [] : [record(run, one.room, seq + 9, at + 4 * 60_000, "run_frozen", { completion: "done" })]),
      ];
    });
  }

  function turn(seq: number, t: number, said: string, notes: readonly Note[], calls: readonly Call[]): Turn {
    return {
      calls,
      notes,
      number: 1,
      opened: Seq.make(seq + 1),
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
    const index = CASES.findIndex((_, at) => runOf(at) === run);
    const one = CASES[index];
    if (one === undefined) return undefined;
    const seq = seqOf(index);
    const t = startOf(index, now) + 1_000;
    return {
      rounds: {
        run,
        turns: [turn(seq, t, one.said, one.notes(seq, t, now), one.calls(seq, t))],
        opened_at: null,
        opening: {
          at: TimeMs.make(startOf(index, now)),
          task: one.task,
          goal: "",
          dispatched_by: "person",
          policy: { mode: "work", write: "full", admit: "standing", landing: "ordinary" },
        },
        worktree: null,
      },
    };
  }

  function queued(now: number): readonly SignalLine[] {
    return [
      { id: "sig-7", from: PLANNER, kind: "thread", first_line: "The parser work landed; the messages can use the byte offsets now.", at: TimeMs.make(now - 180_000) },
      { id: "sig-8", from: "hall/mayor", kind: "mention", first_line: "Keep the old wording for the two errors the docs quote.", at: TimeMs.make(now - 30_000) },
    ];
  }

  export function answering(now: number): (query: Query) => Answer | undefined {
    return (query) => {
      if (query === "preferences") return { preferences: { tags: [] } };
      if (typeof query !== "object") return undefined;
      if ("rounds" in query) return rounds(query.rounds.run, now);
      if ("inbox_view" in query) {
        return { inbox: { addr: query.inbox_view.addr, waiting: query.inbox_view.addr === QUEUED ? queued(now) : [] } };
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

  const SHOWN: readonly (readonly [string, Address])[] = [
    ["agent messages · the User's steer beside a resident's letter, and a pulled signal", LETTERS],
    ["agent messages · a reply wait in progress", WAITING],
    ["agent messages · a reply wait ended by the reply", REPLIED],
    ["agent messages · a reply wait ended by the timeout", TIMED_OUT],
    ["agent messages · a reply wait ended because the room left", LEFT],
    ["agent messages · a handback, finished", FINISHED],
    ["agent messages · a handback, stopped", STOPPED],
    ["agent messages · two queued signals with their first lines", QUEUED],
    ["agent messages · a letter from an older Ledger, without its kind or session", OLDER],
    ["agent messages · send lines: delivered, queued, knocked a new run, and not recorded", SENDS],
  ];
</script>

{#each SHOWN as [label, room] (room)}
  <Case {label} width={760}>
    <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} {answers} {records}>
      <div class="h-[440px] px-wide">
        <Talk address={room} band={false} />
      </div>
    </Stand>
  </Case>
{/each}
