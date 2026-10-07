<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The city's runs board at the two sizes that decide it: a city a
  // person reads whole, with runs waiting for them in two buildings, and
  // a city of 1,200 runs, which is the case that only drawing the rows
  // on screen exists for. The same small city also stands as the
  // building table the city page opens on, at a desktop's width and a
  // phone's, where a row's time bar takes a line of its own, and as the
  // skyline and the results list, both drawn from a fixed run table
  // rather than the live belief, so every window, figure, lamp, flag and
  // row ending is in the picture.
  //
  // **The runs are derived, not typed out.** Every run's building,
  // room, phase and start come from its index, so every screenshot
  // draws the same board, and the few waiting runs sit in buildings
  // whose names would sort them last.

  import type { RunBelief } from "../../core/belief";
  import type { Doing } from "../../core/doing";
  import type { BoardRun } from "../runs/lineage";
  import type { CityAnswer } from "../../wire";
  import { Address, NodeId, RunId, Seq, TimeMs, Tokens, UsdMicros } from "../../wire";

  const NOW = 1_780_000_000_000;
  const MINUTE = 60_000;
  const BUILDINGS = ["atlas", "billing", "docs", "orders", "search", "web"] as const;
  const ROOMS = ["api", "backfill", "ui", "migrations"] as const;
  const TASKS = [
    "backfill the order totals for March",
    "move the search index to the new schema and keep the old one readable until the switch",
    "fix the flaky checkout test",
    "draft the release notes",
    "trim the web bundle under its budget",
  ] as const;
  const DOINGS: readonly Doing[] = [
    { kind: "thinking" },
    { kind: "calling", tool: "read", subject: null },
    { kind: "frozen", completion: null },
    { kind: "unknown" },
    { kind: "thinking" },
    { kind: "frozen", completion: "cancelled" },
    { kind: "frozen", completion: "limit" },
    { kind: "frozen", completion: "done" },
  ];

  function idOf(n: number): string {
    const hex = n.toString(16).padStart(8, "0");
    return `${hex}-0000-4000-8000-${hex}0000`;
  }

  function boardOf(total: number, waiting: readonly number[]): readonly BoardRun[] {
    return Array.from({ length: total }, (_, n): BoardRun => {
      const doing: Doing = waiting.includes(n) ? { kind: "waiting" } : DOINGS[n % DOINGS.length] ?? { kind: "unknown" };
      const started = NOW - ((n * 7) % 80) * MINUTE - (n % 5) * 11_000;
      return {
        run: idOf(n + 1),
        addr: `${BUILDINGS[n % BUILDINGS.length] ?? "atlas"}/${ROOMS[(n >> 1) % ROOMS.length] ?? "api"}`,
        task: n % 9 === 4 ? null : TASKS[n % TASKS.length] ?? null,
        // Half the runs with no task written down carry a goal, which
        // titles them; the other half fall back to their room.
        goal: n % 18 === 4 ? "every clippy warning in the gate crate fixed" : null,
        started,
        ended: doing.kind === "frozen" ? started + ((n % 6) + 1) * 90_000 : null,
        doing,
      };
    });
  }

  const SMALL = boardOf(24, [5, 16]);
  const MANY = boardOf(1200, [5, 16, 611, 1186]);

  // The small board's buildings, and City Hall, which has no runs yet
  // and still has its row.
  const IDLE = { unplanned: { budget: { tokens: Tokens.make(0), usd: UsdMicros.make(0) }, steps: 0 } };
  const CITY: CityAnswer = {
    active: 0,
    frozen: 0,
    halted: [],
    pursuits: [],
    runs: [],
    buildings: ["hall", ...BUILDINGS].map((addr) => ({ addr: Address.make(addr), blocked: [], problems: [], progress: IDLE, ready: 0 })),
  };

  // The small board's runs as the belief holds them, for the drawing
  // and the results list: the same addresses, phases and starts, with a
  // pull request on one finished run and a question on each waiting one.
  const HELD: readonly RunBelief[] = SMALL.map((run, n) => ({
    run: RunId.make(run.run),
    addr: run.addr === null ? null : Address.make(run.addr),
    started: run.started === null ? null : TimeMs.make(run.started),
    task: run.task,
    goal: run.goal,
    lastSeq: Seq.make(1),
    doing: run.doing,
    model: null,
    pr: n === 7 ? "fix/flaky-checkout" : null,
    ask: run.doing.kind === "waiting" ? "write to docs/zh/index.md" : null,
    local: false,
    saying: "",
    thinking: "",
  }));

  // The skyline's city: `docs` has a plan with a stuck share and two
  // blocked lines, `orders` stands under a running pursuit and `web`
  // under a paused one.
  const SKYLINE: CityAnswer = {
    ...CITY,
    pursuits: [
      { addr: Address.make("orders"), goal: "every order total reconciled", state: "running", verdict: { kind: "waiting", in_flight: 1 } },
      { addr: Address.make("web"), goal: "the bundle under its budget", state: "paused", verdict: { kind: "paused" } },
    ],
    buildings: CITY.buildings.map((each) =>
      each.addr === "docs"
        ? {
            ...each,
            blocked: [
              { source: NodeId.make("docs/zh"), line: "waiting for a glossary decision", waiting: 1 },
              { source: NodeId.make("docs/api"), line: "the schema is not published yet", waiting: 1 },
            ],
            progress: { planned: { blocked: 2, blocked_ppb: 250_000_000, done: 3, done_ppb: 375_000_000, total: 8 } },
          }
        : each,
    ),
  };
</script>

<script lang="ts">
  import { say } from "../../core/lang";
  import { bandsOf } from "../../core/results";
  import { ui } from "../../ui";
  import AskMayor from "../city/ask_mayor.svelte";
  import { legendOf } from "../city/legend";
  import Legend from "../city/legend.look.svelte";
  import { resultsLookOf } from "../city/results";
  import Results from "../city/results.look.svelte";
  import { cornerPower } from "../city/shape";
  import { lookOf } from "../city/skyline";
  import Skyline from "../city/skyline.look.svelte";
  import Table from "../city/table.svelte";
  import Board from "../runs/board.svelte";
  import Case from "./case.svelte";

  const { lang } = ui();
  let picked = $state.raw<Address | null>(Address.make("docs"));
  const power = cornerPower(document.documentElement);
  const skyline = $derived(
    lookOf(
      { city: SKYLINE, picked, runsIn: (addr) => HELD.filter((run) => run.addr?.split("/")[0] === addr), lang: $lang, power },
      (addr) => {
        picked = addr;
      },
    ),
  );
</script>

<Case label={say($lang, "gallery_runs_board")} width={1200}>
  <Board runs={SMALL} now={NOW} level={2} />
</Case>
<Case label={say($lang, "gallery_runs_many")} width={1200}>
  <Board runs={MANY} now={NOW} level={2} />
</Case>
<Case label={say($lang, "gallery_city_table")} width={1200}>
  <Table city={CITY} runs={SMALL} now={NOW} {picked} onPick={(addr) => { picked = addr; }} />
</Case>
<Case label={say($lang, "gallery_city_table_narrow")} width={390}>
  <Table city={CITY} runs={SMALL} now={NOW} {picked} onPick={(addr) => { picked = addr; }} />
</Case>
<Case label="city · skyline with runs at work, a stuck building, two pursuits and one picked" width={1200}>
  <Skyline {...skyline} />
  <Legend {...legendOf($lang)} />
</Case>
<Case label="city · results, every way a row can end" width={1200}>
  <Results {...resultsLookOf(bandsOf(HELD, NOW), $lang)} />
</Case>
<Case label="city · nothing asked yet, the way to the Mayor">
  <AskMayor />
</Case>
