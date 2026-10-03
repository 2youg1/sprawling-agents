<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // Who handed a room its work (client D83): a child room opened by a
  // resident, its task signed with that resident and linked to the run
  // that delegated it (`Opening.parent`); a room the User dispatched,
  // signed "you"; and one the city's desk dispatched. Below them, the
  // sessions pane with the parent's row and the child's, which says
  // which resident delegated it (`SessionStart::Dispatched { by }`).
  // Each room holds one run, so each thread shows only its own case.

  import type { Answer, EventKind, EventRecord, Query, SessionLine, SessionStart, Turn } from "../../wire";
  import { Address, B3Hash, RunId, Seq, TimeMs, Tokens } from "../../wire";

  const MODEL = "anthropic/claude-sonnet-5";

  export const PARENT = Address.make("lab/planner");
  export const CHILD = Address.make("lab/parser");
  export const BY_USER = Address.make("lab/docs");
  export const BY_CITY = Address.make("lab/sweep");

  interface Scene {
    readonly room: Address;
    readonly run: RunId;
    readonly seq: number;
    readonly task: string;
    readonly said: string;
    // The `Who` spelling `run_started.dispatched_by` records.
    readonly by: string;
    readonly parent: RunId | null;
    readonly start: SessionStart;
  }

  const PARENT_RUN = RunId.make("0199c0de-c41d-4000-8000-000000000001");

  const SCENES: readonly Scene[] = [
    {
      room: PARENT,
      run: PARENT_RUN,
      seq: 10,
      task: "Split the parser work: keep the grammar here, hand the error positions to a room of their own.",
      said: "The error positions are with lab/parser now; the grammar stays here.",
      by: "person",
      parent: null,
      start: { opened: { carry: "nothing", from: null } },
    },
    {
      room: CHILD,
      run: RunId.make("0199c0de-c41d-4000-8000-000000000002"),
      seq: 20,
      task: "Report every parse error as a byte offset from the start of the input, and say when the tests pass.",
      said: "Error positions are byte offsets on the grammar's `Span`, and the parser tests pass.",
      by: PARENT,
      parent: PARENT_RUN,
      start: { dispatched: { by: PARENT } },
    },
    {
      room: BY_USER,
      run: RunId.make("0199c0de-c41d-4000-8000-000000000003"),
      seq: 30,
      task: "Write the error-format section of the docs.",
      said: "The section names the offset base and gives one example per error.",
      by: "person",
      parent: null,
      start: { dispatched: { by: "person" } },
    },
    {
      room: BY_CITY,
      run: RunId.make("0199c0de-c41d-4000-8000-000000000004"),
      seq: 40,
      task: "Sweep the worktrees left behind by finished runs.",
      said: "Three worktrees were left behind; all three are removed.",
      by: "city",
      parent: null,
      start: { dispatched: { by: "city" } },
    },
  ];

  function startOf(seq: number, now: number): number {
    return now - (50 - seq) * 30_000;
  }

  function record(scene: Scene, seq: number, at: number, kind: EventKind, data: Record<string, unknown>): EventRecord {
    return { run: scene.run, seq: Seq.make(seq), kind, t: TimeMs.make(at), who: "city", addr: scene.room, prev: B3Hash.make("0".repeat(64)), v: 1, data };
  }

  export function recordsAt(now: number): readonly EventRecord[] {
    return SCENES.flatMap((scene) => {
      const at = startOf(scene.seq, now);
      return [
        record(scene, scene.seq, at, "run_started", { task: scene.task, dispatched_by: scene.by }),
        record(scene, scene.seq + 1, at + 1_000, "model_called", { model: MODEL }),
        record(scene, scene.seq + 2, at + 60_000, "run_frozen", { completion: "done" }),
      ];
    });
  }

  function turn(scene: Scene, t: number): Turn {
    return {
      calls: [],
      notes: [],
      number: 1,
      opened: Seq.make(scene.seq + 1),
      t: TimeMs.make(t),
      timing: "measured",
      first_at: TimeMs.make(t + 450),
      model: MODEL,
      said: scene.said,
      used: { input: Tokens.make(12_000), output: Tokens.make(400), cached: Tokens.make(6_000) },
      stopped: "end_turn",
    };
  }

  function rounds(run: RunId, now: number): Answer | undefined {
    const scene = SCENES.find((each) => each.run === run);
    if (scene === undefined) return undefined;
    const at = startOf(scene.seq, now);
    return {
      rounds: {
        run,
        turns: [turn(scene, at + 1_000)],
        opened_at: null,
        opening: {
          at: TimeMs.make(at),
          task: scene.task,
          goal: "",
          dispatched_by: scene.by,
          parent: scene.parent,
          policy: { mode: "work", write: "full", admit: "standing", landing: "ordinary" },
        },
        worktree: null,
      },
    };
  }

  function sessionsOf(room: string, now: number): SessionLine[] {
    return SCENES.filter((scene) => scene.room === room).map((scene) => ({
      began: Seq.make(scene.seq),
      last: Seq.make(scene.seq + 2),
      at: TimeMs.make(startOf(scene.seq, now) + 60_000),
      runs: 1,
      model: MODEL,
      preview: scene.said,
      start: scene.start,
    }));
  }

  export function answering(now: number): (query: Query) => Answer | undefined {
    return (query) => {
      if (query === "preferences") return { preferences: { tags: [] } };
      if (typeof query !== "object") return undefined;
      if ("rounds" in query) return rounds(query.rounds.run, now);
      if ("inbox_view" in query) return { inbox: { addr: query.inbox_view.addr, waiting: [] } };
      if ("sessions" in query) return { sessions: { room: query.sessions.room, sessions: sessionsOf(query.sessions.room, now), earlier: 0 } };
      return undefined;
    };
  }
</script>

<script lang="ts">
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import Talk from "../talk.svelte";
  import Sessions from "../world/sessions.svelte";
  import Case from "./case.svelte";
  import Stand from "./stand.svelte";

  const u = ui();
  const { lang } = u;
  const now = u.now();
  const records = recordsAt(now);
  const answers = answering(now);

  const SHOWN: readonly (readonly [string, Address])[] = [
    ["agent messages · a child room's opening, delegated by a resident and linked to its run", CHILD],
    ["agent messages · a room's opening, dispatched by the User", BY_USER],
    ["agent messages · a room's opening, dispatched by the city", BY_CITY],
  ];
</script>

{#snippet head()}
  <h2 class="flex h-control shrink-0 items-center text-note text-text-faint">{say($lang, "world_sessions")}</h2>
{/snippet}

{#each SHOWN as [label, room] (room)}
  <Case {label} width={760}>
    <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} {answers} {records}>
      <div class="h-[320px] px-wide">
        <Talk address={room} band={false} />
      </div>
    </Stand>
  </Case>
{/each}

<Case label="agent messages · session rows of a parent and the child it delegated" width={360}>
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} {answers} {records}>
    <div class="flex h-[480px] flex-col">
      <Sessions here={CHILD} narrow={false} {head} />
    </div>
  </Stand>
</Case>
