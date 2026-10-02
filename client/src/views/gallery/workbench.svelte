<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts" module>
  // The world layer as client-SPEC 7H and 7K draw it, in a made-up city
  // of three buildings: the panorama workbench on a building room with a
  // commit picked, the same room in the blend tier, the workbench with
  // the right pane open, and the two readings of the third pane that
  // follow the workspace - the city beside the Mayor's room and the
  // files beside a building's.
  //
  // The room's session ran three turns that read, ran and edited, and
  // checkpointed twice; its building's history holds a branch that
  // another room's session merged back, so the swimlane graph has a
  // lane leaving and rejoining. The session's figures are chosen to be
  // told apart: 1.2 million tokens read, a fifth of them from the cache,
  // a median time to first content of 412 ms.
  import type { Answer, Call, CityAnswer, CommitAnswer, EndpointsAnswer, EventKind, EventRecord, Query, Turn } from "../../wire";
  import { Address, B3Hash, GitOid, RunId, Seq, TimeMs, Tokens, UsdMicros, Window } from "../../wire";
  import { pickCommit } from "../world/chosen.svelte";

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
    return RUNS.flatMap(([room, task, ending], n) => {
      const began = now - (RUNS.length - n) * 9 * 60_000;
      const opened = record(n, began, "run_started", room, { task });
      if (ending === null) return [opened];
      const data = ending === "run_frozen" ? { completion: "done" } : { action_desc: "run cargo publish --dry-run" };
      return [opened, record(n, began + 120_000, ending, room, data)];
    });
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
    readonly used: readonly [number, number, number];
  }

  const SHAPES: readonly Shape[] = [
    { calls: [call(11, "read", "crates/city/city-SPEC.md", 31), call(12, "exec", "cargo nextest -p city", 3_412)], notes: [], first: 398, used: [380_000, 900, 61_000] },
    { calls: [call(21, "edit", "crates/city/src/document.rs", 87)], notes: [{ checkpointed: { at: Seq.make(22), oid: oid("d") } }], first: 412, used: [402_000, 1_100, 88_000] },
    { calls: [call(31, "edit", "crates/city/city-SPEC.md", 64), call(33, "exec", "just check-client", null)], notes: [{ checkpointed: { at: Seq.make(32), oid: oid("e") } }], first: 455, used: [418_000, 1_300, 92_000] },
  ];

  const TURNS: readonly Turn[] = SHAPES.map(({ calls, notes, first, used }, index) => ({
    calls,
    notes,
    number: index + 1,
    opened: Seq.make(10 * (index + 1)),
    t: TimeMs.make(START + (index * 10 + 10) * 1_000),
    first_at: TimeMs.make(START + (index * 10 + 10) * 1_000 + first),
    timing: "measured",
    model: MODEL,
    said: "Done.",
    used: { input: Tokens.make(used[0]), output: Tokens.make(used[1]), cached: Tokens.make(used[2]) },
    spent: UsdMicros.make(140_000),
  }));

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
    if ("rounds" in query) return { rounds: { run: query.rounds.run, turns: query.rounds.run === run(6) ? TURNS : [], opened_at: oid("a") } };
    if ("commits" in query) return { commits: { building: null, before: null, commits: COMMITS, more: true } };
    if ("config" in query) {
      return {
        config: {
          addr: query.config.addr,
          second: { percent: 65, domain: { min: 31, max: 90 }, from: "city" },
          tuning: { from: "default", proxying: "except_local", timeout_ms: 600_000 },
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
  import type { Tier } from "../../core/prefs";
  import { ui } from "../../ui";
  import Edge from "../edge.svelte";
  import Workspace from "../workspace.svelte";
  import City from "../world/city.svelte";
  import Files from "../world/files.svelte";
  import Case from "./case.svelte";
  import Stand from "./stand.svelte";

  const records = recordsAt(ui().now());

  interface Shown {
    readonly label: string;
    readonly tier: Tier;
    readonly panel: boolean;
  }

  const SHOWN: readonly Shown[] = [
    { label: "workbench · panorama, a building room with a commit picked", tier: "panorama", panel: false },
    { label: "workbench · blend, beside the conversation", tier: "blend", panel: false },
    { label: "workbench · panorama, the right pane open", tier: "panorama", panel: true },
  ];
</script>

{#each SHOWN as shown (shown.label)}
  <Case label={shown.label} width={1440}>
    <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={answering} {records}>
      <div class="frame relative h-[860px] translate-x-0 overflow-hidden bg-page">
        <Workspace address={ROOM} tier={shown.tier} seat="specimen" panel={shown.panel} />
        <Edge tier={shown.tier} onTier={() => undefined} onPeek={() => undefined} mailboxAsked={0} />
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
