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
  // phone's, where a row's time bar takes a line of its own.
  //
  // **The runs are derived, not typed out.** Every run's building,
  // room, phase and start come from its index, so every screenshot
  // draws the same board, and the few waiting runs sit in buildings
  // whose names would sort them last.

  import type { Doing } from "../../core/doing";
  import type { BoardRun } from "../runs/lineage";
  import type { CityAnswer } from "../../wire";
  import { Address, Tokens, UsdMicros } from "../../wire";

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
</script>

<script lang="ts">
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import Table from "../city/table.svelte";
  import Board from "../runs/board.svelte";
  import Case from "./case.svelte";

  const { lang } = ui();
  let picked = $state.raw<Address | null>(Address.make("docs"));
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
