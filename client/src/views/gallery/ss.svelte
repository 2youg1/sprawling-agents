<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts" module>
  // The sessions pane with a row per session (client/Spec.lean §7K), in a
  // made-up city of three buildings: the Mayor's current session pinned
  // by being current, an earlier session of `lab/room1` pinned by its tag,
  // tags on four sessions, and the pane filtered by one of them; the row
  // menu held open, on an earlier session and on the current one, whose
  // menu also renames it and changes its run policy; and an earlier
  // session in main, beside the pane in the blend tier and under the session pane in the panorama tier, with
  // the way back to the room's current session.
  import type { Answer, EventKind, EventRecord, Query, SessionLine, SessionTags, Turn } from "../../wire";
  import { Address, B3Hash, RunId, Seq, Tag, TimeMs } from "../../wire";

  const CITY = Address.make("sprawling");
  const ROOM = Address.make("lab/room1");
  export const PAST = Seq.make(10);
  const run = (n: number): RunId => RunId.make(`0199c0de-0000-4000-8000-5500000000${n.toString(16).padStart(2, "0")}`);

  // Each run: its number, its room, its task, the line it started on,
  // and how it stands (`null` still going).
  const RUNS: readonly (readonly [number, string, string, number, EventKind | null])[] = [
    [1, "hall/mayor", "plan the week", 2, "run_frozen"],
    [2, "lab/room1", "read the city's document contract", 4, "run_frozen"],
    [3, "lab/room1", "the parser's error positions", 10, "run_frozen"],
    [4, "lab/room2", "nextest coverage for the city", 12, "run_frozen"],
    [5, "shop/back", "rewrite the sieve's cut threshold", 14, "approval_requested"],
    [6, "hall/mayor", "what is left before the release", 20, "run_frozen"],
    [7, "lab/room1", "change the city's document reading contract", 30, null],
  ];

  function record(n: number, seq: number, at: number, kind: EventKind, room: string, data: Record<string, unknown>): EventRecord {
    return { run: run(n), seq: Seq.make(seq), kind, t: TimeMs.make(at), who: "city", addr: Address.make(room), prev: B3Hash.make("0".repeat(64)), v: 1, data };
  }

  export function recordsAt(now: number): readonly EventRecord[] {
    const named = { ...record(0, 0, now - 7_200_000, "city_initialized", "sprawling", {}), run: RunId.make("00000000-0000-0000-0000-000000000000") };
    return [
      named,
      ...RUNS.flatMap(([n, room, task, seq, ending]) => {
        const at = now - (40 - seq) * 120_000;
        const started = record(n, seq, at, "run_started", room, { task });
        if (ending === null) return [started];
        const data = ending === "run_frozen" ? { completion: "done" } : { action_desc: "run cargo publish --dry-run" };
        return [started, record(n, seq + 1, at + 90_000, ending, room, data)];
      }),
    ];
  }

  // What the city says of a session beyond its lines: a display name on
  // one, and the model, effort, workspace and last reply on most, so a
  // row with none of them sits beside rows with all of them.
  const SAID: Readonly<Record<number, Partial<SessionLine>>> = {
    30: { name: "document contract", model: "claude-opus-4-1", effort: "high", workspace: "lab-1", preview: "The reading contract now names every field the document page draws, and the three it used to guess are asked of the city." },
    10: { model: "claude-sonnet-4-5", effort: "medium", workspace: "lab-1", preview: "Error positions are byte offsets from the start of the input." },
    12: { model: "gpt-5", workspace: "lab-2", preview: "Coverage is 81 % of lines." },
    14: { model: "claude-sonnet-4-5", effort: "low", preview: "Waiting for the User to allow the dry run." },
  };

  // The rooms' stretches, newest first, as `Query::Sessions` answers.
  function linesOf(room: string, now: number): SessionLine[] {
    return RUNS.filter(([, at]) => at === room)
      .map(([, , , seq], index): SessionLine => ({
        ...SAID[seq],
        began: Seq.make(seq),
        last: Seq.make(seq + 1),
        at: TimeMs.make(now - (40 - seq) * 120_000 + 90_000),
        runs: 1,
        start: index === 0 ? { dispatched: { by: null } } : { opened: { carry: index === 1 ? "handoff" : "nothing", from: null } },
      }))
      .reverse();
  }

  const TAGS: readonly SessionTags[] = [
    { city: CITY, room: ROOM, began: PAST, tags: [Tag.make("parser"), Tag.make("pin")] },
    { city: CITY, room: Address.make("lab/room2"), began: Seq.make(12), tags: [Tag.make("later")] },
    { city: CITY, room: Address.make("shop/back"), began: Seq.make(14), tags: [Tag.make("parser")] },
    { city: CITY, room: Address.make("hall/mayor"), began: Seq.make(2), tags: [Tag.make("later")] },
  ];

  function turn(n: number, said: string, now: number): Turn {
    return { calls: [], notes: [], number: 1, opened: Seq.make(n * 10), t: TimeMs.make(now), timing: "measured", said };
  }

  export function answering(now: number): (query: Query) => Answer | undefined {
    return (query) => {
      if (query === "preferences") return { preferences: { tags: TAGS } };
      if (typeof query !== "object") return undefined;
      if ("sessions" in query) return { sessions: { room: query.sessions.room, sessions: linesOf(query.sessions.room, now), earlier: 0 } };
      if ("rounds" in query) {
        const found = RUNS.find(([n]) => run(n) === query.rounds.run);
        const said = found === undefined ? "" : `The work on ${found[2]} is written down in the room's Memo.md.`;
        return { rounds: { run: query.rounds.run, turns: [turn(found?.[0] ?? 0, said, now)], opened_at: null, opening: { at: TimeMs.make(now), goal: "", task: found?.[2] ?? "", policy: { mode: "work", write: "full", admit: "standing", landing: "ordinary" } }, worktree: null } };
      }
      return undefined;
    };
  }
</script>

<script lang="ts">
  import { say } from "../../core/lang";
  import type { Tier } from "../../core/prefs";
  import { ui } from "../../ui";
  import Workspace from "../workspace.svelte";
  import SessionMenu from "../world/session_menu.svelte";
  import Sessions from "../world/sessions.svelte";
  import Case from "./case.svelte";
  import { pressing } from "./pressing";
  import Stand from "./stand.svelte";

  const u = ui();
  const { lang } = u;
  const now = u.now();
  const records = recordsAt(now);
  const answers = answering(now);
  const MAINS: readonly (readonly [string, Tier])[] = [
    ["sessions · an earlier session in main, blend, the way back to the current session", "blend"],
    ["sessions · an earlier session in main, panorama, the way back to the current session", "panorama"],
  ];
</script>

{#snippet head()}
  <h2 class="flex h-control shrink-0 items-center text-note text-text-faint">{say($lang, "world_sessions")}</h2>
{/snippet}

<Case label="sessions · a pinned group, tags, and the pane filtered by one" width={360}>
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} {answers} {records}>
    <div class="flex h-[560px] flex-col" {@attach pressing('[role="group"] button:nth-of-type(4)')}>
      <Sessions here={ROOM} narrow={false} {head} />
    </div>
  </Stand>
</Case>
<Case label="sessions · every session, the row menu open" width={360}>
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} {answers} {records}>
    <div class="flex h-[640px] flex-col" {@attach pressing('ul li:nth-of-type(2) button[aria-haspopup="menu"]')}>
      <Sessions here={ROOM} narrow={false} {head} />
    </div>
  </Stand>
</Case>
<Case label="sessions · the current session's menu: rename, mode and write limit" width={360}>
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} {answers} {records}>
    <div class="flex h-[640px] flex-col" {@attach pressing('li:has(a[aria-current="page"]) button[aria-haspopup="menu"]')}>
      <Sessions here={ROOM} narrow={false} {head} />
    </div>
  </Stand>
</Case>
<Case label="sessions · a row's menu key while the city has no name" width={360}>
  <div class="flex justify-end p-base">
    <SessionMenu named={null} label="room1" tags={[]} pinning="none" session={{ name: "", run: null }} />
  </div>
</Case>
{#each MAINS as [label, tier] (label)}
  <Case {label} width={1440}>
    <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} {answers} {records}>
      <!-- A container named as the shell's body is, so the specimen
      folds to one column on its own width rather than the window's. -->
      <div class="@container/shell">
        <div class="frame relative h-[760px] translate-x-0 overflow-hidden bg-page">
          <Workspace address={ROOM} session={PAST} {tier} seat="specimen" panel={false} />
        </div>
      </div>
    </Stand>
  </Case>
{/each}
