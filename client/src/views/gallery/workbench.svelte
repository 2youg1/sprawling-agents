<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts" module>
  // The world layer as docs/frontend-method.md §7H and client/Spec.lean §7K draw it, in a made-up city
  // of three buildings: the panorama workbench on a building room with a
  // commit picked, the same room in the blend tier, the workbench with
  // the right pane open, and the two readings of the third pane that
  // follow the workspace - the city beside the Mayor's room and the
  // files beside a building's - and, closer, what the wire's run-level
  // fields draw: the room chip open on who is listening, the session
  // pane at the width a panorama gives it, and a picked commit's B3.
  // The panorama is drawn at 1440 and at 1920, and the instrument sheet
  // alone at a phone's 390, because the sheet's two speed cells must
  // hold their figures uncut at each of the three.
  //
  // The room's session ran three turns that read, ran and edited, and
  // checkpointed twice; its building's history holds a branch that
  // another room's session merged back, so the swimlane graph has a
  // lane leaving and rejoining. The session's figures are chosen to be
  // told apart: 1.2 million tokens read, a fifth of them from the cache,
  // a median time to first content of 412 ms. The run works in its own
  // worktree under the tested-and-ordinary policy, its finished command
  // exited 0, and each commit carries the B3 of its checkpoint line.
  import type { Answer, Call, CityAnswer, CommitAnswer, EndpointsAnswer, EventKind, EventRecord, Opening, PrefixSlot, Query, Turn } from "../../wire";
  import { Address, B3Hash, GitOid, RunId, Seq, TimeMs, Tokens, UsdMicros, Window } from "../../wire";
  import { pickCommit } from "../world/chosen.svelte";
  import { SEARCH, UNTUNED } from "./configured";

  export const ROOM = Address.make("lab/room1");
  const MODEL = "anthropic/claude-fable-5.1";
  const START = 1_790_000_000_000;
  const run = (n: number): RunId => RunId.make(`0199c0de-0000-4000-8000-${n.toString(16).padStart(12, "0")}`);
  const oid = (digit: string): GitOid => GitOid.make(digit.repeat(40));

  // Who ran where, and how each run stands: still going, waiting for the
  // person, or frozen.
  const RUNS: readonly (readonly [string, string, EventKind | null])[] = [
    ["lab/room1", "read the city's document contract", "run_frozen"],
    ["lab/room2", "nextest coverage for the city", "run_frozen"],
    ["lab/room3", "proposal card: move documents", "approval_requested"],
    ["shop/back", "rewrite the sieve's cut threshold", null],
    ["shop/front", "README badges as HTML", "run_frozen"],
    ["hall/mayor", "plan the week", "run_frozen"],
    ["lab/room1", "change the city's document reading contract", null],
  ];

  function record(n: number, at: number, kind: EventKind, room: string, data: Record<string, unknown>): EventRecord {
    return {
      run: run(n),
      seq: Seq.make(n * 2 + (kind === "run_started" ? 1 : 2)),
      kind,
      t: TimeMs.make(at),
      who: "city",
      addr: Address.make(room),
      prev: B3Hash.make("0".repeat(64)),
      v: 1,
      data,
    };
  }

  export function recordsAt(now: number): readonly EventRecord[] {
    const named = { ...record(127, now - 3_600_000, "city_initialized", "sprawling", {}), seq: Seq.make(0) };
    return [named, ...RUNS.flatMap(([room, task, ending], n) => {
      const began = now - (RUNS.length - n) * 9 * 60_000;
      const opened = record(n, began, "run_started", room, { task });
      if (ending === null) return [opened];
      const data = ending === "run_frozen" ? { completion: "done" } : { action_desc: "run cargo publish --dry-run" };
      return [opened, record(n, began + 120_000, ending, room, data)];
    })];
  }

  function call(at: number, tool: string, subject: string, ms: number | null): Call {
    return {
      tool,
      subject,
      arguments: null,
      outcome: ms === null ? "waiting" : "answered",
      at: Seq.make(at),
      output: { head: "ok", cut: 0 },
      called: TimeMs.make(START + at * 1_000),
      answered: ms === null ? null : TimeMs.make(START + at * 1_000 + ms),
      timing: "measured",
      exit_code: tool === "exec" && ms !== null ? 0 : null,
    };
  }

  // The commits of building `lab`, newest first. The fifth digit is a
  // commit the page does not hold, so the oldest lane runs off the foot.
  // Each commit's time is seconds after the session's start: the two
  // the live session made land on its two checkpoints.
  const HISTORY: readonly (readonly [string, readonly string[], string, number, string, number])[] = [
    ["e", ["d"], "edit document.rs", 6, "lab/room1", 32.4],
    ["d", ["c"], "read the specification", 6, "lab/room1", 22.2],
    ["c", ["a", "b"], "merge room2 into lab", 1, "lab/room2", -600],
    ["b", ["a"], "nextest coverage for the city", 1, "lab/room2", -1_200],
    ["a", ["5"], "the document contract, first draft", 0, "lab/room1", -1_800],
  ];

  const COMMITS: readonly CommitAnswer[] = HISTORY.map(([digit, parents, message, n, actor, after], row) => ({
    actor: Address.make(actor),
    at: TimeMs.make(START + after * 1_000),
    lineage: [],
    message,
    model: MODEL,
    oid: oid(digit),
    b3: B3Hash.make(`9f2c41ade07b5c1e${digit}`.repeat(4).slice(0, 64)),
    effort: "medium",
    parents: parents.map(oid),
    run: run(n),
    seq: Seq.make(1_000 - row),
    spent: UsdMicros.make(12_000),
  }));

  // The live session's three turns: it read and ran, edited and
  // checkpointed, then checkpointed again and is running the tests.
  interface Shape {
    readonly calls: readonly Call[];
    readonly notes: Turn["notes"];
    readonly first: number;
    readonly used: readonly [number, number, number, number];
  }

  const SHAPES: readonly Shape[] = [
    { calls: [call(11, "read", "crates/city/city-SPEC.md", 31), call(12, "exec", "cargo nextest -p city", 3_412)], notes: [], first: 398, used: [380_000, 900, 61_000, 22_400] },
    { calls: [call(21, "edit", "crates/city/src/document.rs", 87)], notes: [{ checkpointed: { at: Seq.make(22), oid: oid("d") } }], first: 412, used: [402_000, 1_100, 88_000, 9_800] },
    { calls: [call(31, "edit", "crates/city/city-SPEC.md", 64), call(33, "exec", "just check-client", null)], notes: [{ checkpointed: { at: Seq.make(32), oid: oid("e") } }], first: 455, used: [418_000, 1_300, 92_000, 9_900] },
  ];

  const TURNS: readonly Turn[] = SHAPES.map(({ calls, notes, first, used }, index) => ({
    calls,
    notes,
    number: index + 1,
    opened: Seq.make(10 * (index + 1)),
    t: TimeMs.make(START + (index * 10 + 10) * 1_000),
    first_at: TimeMs.make(START + (index * 10 + 10) * 1_000 + first),
    returned: TimeMs.make(START + (index * 10 + 10) * 1_000 + first + 18_000),
    timing: "measured",
    model: MODEL,
    said: "Done.",
    used: { input: Tokens.make(used[0]), output: Tokens.make(used[1]), cached: Tokens.make(used[2]), cache_write: Tokens.make(used[3]) },
    spent: UsdMicros.make(140_000),
  }));

  const OPENING: Opening = {
    at: TimeMs.make(START),
    goal: "",
    task: "change the city's document reading contract",
    policy: { admit: "tested", landing: "ordinary", mode: "work", write: "full" },
    effort: "medium",
  };

  // What the live run was told, segment by segment; the building's
  // segment lost the end of its file to the budget.
  const SEGMENTS: readonly (readonly [PrefixSlot, number, string, number])[] = [
    ["city", 2_140, "CITY.md", 0],
    ["building", 6_812, "lab/AGENTS.md", 1_204],
    ["resident", 1_390, "lab/room1/RESIDENT.md", 0],
    ["run", 620, "lab/room1/TASK.md", 0],
  ];

  const ENDPOINTS: EndpointsAnswer = {
    chosen: [{ endpoint: "zenmux", model: MODEL, tag: "main" }],
    endpoints: [
      {
        base_url: "https://api.zenmux.ai/v1",
        connection_kind: "openai_compat",
        dialect: "open_ai",
        has_credential: true,
        label: "ZenMux",
        local: false,
        models: [
          {
            id: MODEL,
            context_tokens: Window.make(1_000_000),
            input_modalities: ["text"],
            input_price: "$3/M",
            output_price: "$15/M",
          },
        ],
        name: "zenmux",
        tuning: UNTUNED,
        account_status: [],
      },
    ],
  };

  const unplanned = { unplanned: { budget: { tokens: Tokens.make(0), usd: UsdMicros.make(0) }, steps: 3 } };
  const CITY: CityAnswer = {
    active: 2,
    frozen: 4,
    halted: [],
    pursuits: [],
    runs: [],
    buildings: ["hall", "lab", "shop"].map((name) => ({ addr: Address.make(name), blocked: [], problems: [], progress: unplanned, ready: 0 })),
  };

  function answering(query: Query): Answer | undefined {
    if (query === "cost_view") {
      return {
        cost: {
          by_actor: [],
          by_run: [[run(6), UsdMicros.make(410_000)]],
          by_segment: [],
          by_skill: [],
          by_tool: [],
          total: UsdMicros.make(1_200_000),
          unpriced: { calls: 0, tokens: 0 },
        },
      };
    }
    if (query === "endpoint_view") return { endpoints: ENDPOINTS };
    if (query === "city_view") return { city: CITY };
    if (query === "governance") return { governance: { autonomy: "owner", decided: [] } };
    if (typeof query !== "object") return undefined;
    if ("rounds" in query) {
      const live = query.rounds.run === run(6);
      return { rounds: { run: query.rounds.run, turns: live ? TURNS : [], opened_at: oid("a"), opening: live ? OPENING : null, worktree: live ? "lab-room1" : null } };
    }
    if ("commits" in query) return { commits: { building: null, before: null, commits: COMMITS, more: true } };
    if ("config" in query) {
      return {
        config: {
          addr: query.config.addr,
          first: 30,
          second: { percent: 65, domain: { min: 31, max: 90 }, from: "city" },
          tuning: { from: "default", proxying: "except_local", timeout_ms: 600_000, account_retries: "two" },
          search: SEARCH,
        },
      };
    }
    if ("changes" in query) {
      return {
        changes: {
          base: query.changes.base,
          head: query.changes.head ?? null,
          files: [
            { path: "crates/city/src/document.rs", how: "modified", lines: { counted: { added: 12, removed: 3 } } },
            { path: "crates/city/city-SPEC.md", how: "modified", lines: { counted: { added: 4, removed: 1 } } },
          ],
        },
      };
    }
    // Files only, and names of one length: `building/tree.svelte` draws a
    // file row's glyph box with no height, so `cargo xtask render` takes
    // the name's centre for the row's first mark, and a folder at the top
    // opens by itself into rows indented off that line. Both are the
    // tree's to fix; this fixture draws the rows the tree draws straight.
    if ("prefix" in query) {
      const segments = SEGMENTS.map(([slot, bytes, addr, dropped], at) => ({
        slot,
        bytes,
        hash: B3Hash.make(String(at + 1).repeat(64)),
        sources: [{ addr: Address.make(addr), kept: bytes, dropped }],
        stored: true,
        text: "",
      }));
      return { prefix: { run: query.prefix.run, segments } };
    }
    if ("listing" in query) {
      return {
        listing: {
          at: query.listing.at ?? null,
          entries: [
            { name: "Memo.md", kind: { file: { bytes: 812 } } },
            { name: "Plan.md", kind: { file: { bytes: 2_140 } } },
            { name: "SPEC.md", kind: { file: { bytes: 18_400 } } },
          ],
        },
      };
    }
    return undefined;
  }

  // The commit the first case shows picked: the session's own last
  // checkpoint, opened under its row.
  const PICKED = COMMITS[0];
  if (PICKED !== undefined) pickCommit(PICKED);
</script>

<script lang="ts">
  import { say } from "../../core/lang";
  import { roomOf } from "../../core/route";
  import type { Tier } from "../../core/prefs";
  import { ui } from "../../ui";
  import Edge from "../edge.svelte";
  import Listening from "../talk/listening.svelte";
  import PillView from "../talk/pill.svelte";
  import Workspace from "../workspace.svelte";
  import City from "../world/city.svelte";
  import Commits from "../world/commits.svelte";
  import Files from "../world/files.svelte";
  import Session from "../world/session.svelte";
  import Sheet from "../world/sheet.svelte";
  import Case from "./case.svelte";
  import Stand from "./stand.svelte";

  const u = ui();
  const { lang } = u;
  const records = recordsAt(u.now());
  // The room chip as the settings row draws it, held open.
  const chip = $derived({
    label: say($lang, "talk_column_workspace"),
    placeholder: say($lang, "talk_column_workspace"),
    choices: ["hall/mayor", "lab/room1", "lab/room2", "shop/back"].map((room) => ({ value: room, label: room })),
    value: ROOM,
    pick: () => undefined,
  });

  interface Shown {
    readonly label: string;
    readonly tier: Tier;
    readonly panel: boolean;
    readonly width: number;
  }

  const SHOWN: readonly Shown[] = [
    { label: "workbench · panorama, a building room with a commit picked", tier: "panorama", panel: false, width: 1440 },
    { label: "workbench · panorama at 1920", tier: "panorama", panel: false, width: 1920 },
    { label: "workbench · blend, beside the conversation", tier: "blend", panel: false, width: 1440 },
    { label: "workbench · panorama, the right pane open", tier: "panorama", panel: true, width: 1440 },
  ];
</script>

{#each SHOWN as shown (shown.label)}
  <Case label={shown.label} width={shown.width}>
    <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={answering} {records}>
      <!-- A container named as the shell's body is, so the specimen's
      silver cut is measured on its own width. -->
      <div class="@container/shell">
        <div class="frame relative h-[860px] translate-x-0 overflow-hidden bg-page">
          <Workspace address={ROOM} tier={shown.tier} seat="specimen" panel={shown.panel} />
          <Edge tier={shown.tier} onTier={() => undefined} onPeek={() => undefined} mailboxAsked={0} />
        </div>
      </div>
    </Stand>
  </Case>
{/each}
<Case label="workbench · the city beside the Mayor's room">
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={answering} {records}>
    <div class="flex h-[360px] w-[440px] flex-col">
      <City />
    </div>
  </Stand>
</Case>
<Case label="workbench · a building's files beside its room">
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={answering} {records}>
    <div class="flex h-[360px] w-[440px] flex-col">
      <Files here={ROOM} />
    </div>
  </Stand>
</Case>
{#snippet bare()}{/snippet}
{#snippet listening()}<Listening room={ROOM} />{/snippet}
<Case label="workbench · the room chip open on who is listening">
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={answering} {records}>
    <div class="flex h-[540px] items-end pb-base">
      <PillView spec={chip} starts="open" told={listening} />
    </div>
  </Stand>
</Case>
<Case label="workbench · the chosen session's pane at its panorama width" width={640}>
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={answering} {records}>
    <div class="flex h-[640px] flex-col">
      <Session here={ROOM} title={roomOf(ROOM)} head={bare} />
    </div>
  </Stand>
</Case>
<Case label="workbench · the session's instrument sheet at a phone's width" width={390}>
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={answering} {records}>
    <Sheet here={ROOM} run={run(6)} rounds={{ run: run(6), turns: TURNS, opened_at: oid("a"), opening: OPENING, worktree: "lab-room1" }} />
  </Stand>
</Case>
<Case label="workbench · a picked commit's identity in the commits pane">
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={answering} {records}>
    <div class="flex h-[520px] w-[440px] flex-col">
      <Commits here={ROOM} />
    </div>
  </Stand>
</Case>
