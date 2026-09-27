// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { Address, Tokens, UsdMicros } from "../../wire";
import type { BuildingProgress } from "../../wire";
import type { BoardRun } from "../runs/lineage";
import { tableOf } from "./table";

const IDLE = { unplanned: { budget: { tokens: Tokens.make(0), usd: UsdMicros.make(0) }, steps: 0 } };

function building(addr: string): BuildingProgress {
  return { addr: Address.make(addr), blocked: [], problems: [], progress: IDLE, ready: 0 };
}

function run(id: string, addr: string, started: number, doing: BoardRun["doing"]): BoardRun {
  return { run: id, addr, task: null, goal: null, started, ended: null, doing };
}

describe("building table", () => {
  // A building holding a run that waits for the person leads the table,
  // the rest follow by name; a building with no runs still has its row,
  // and a run outside every building the city names is left to the
  // board, which lists every run.
  test("the table counts each building's runs and puts the one waiting first", () => {
    const rows = tableOf(
      [building("hall"), building("docs"), building("shop")],
      [
        run("r1", "shop/checkout", 1_000, { kind: "thinking" }),
        run("r2", "shop/admin", 3_000, { kind: "waiting" }),
        run("r3", "docs/zh", 2_000, { kind: "frozen", completion: null }),
        run("r4", "docs", 4_000, { kind: "unknown" }),
        run("r5", "gone/room", 5_000, { kind: "thinking" }),
      ],
    );
    expect(rows).toEqual([
      {
        addr: Address.make("shop"),
        guide: "├─",
        waiting: 1,
        working: 1,
        done: 0,
        latest: 3_000,
        since: 1_000,
        starts: [
          { run: "r1", at: 1_000, phase: "model" },
          { run: "r2", at: 3_000, phase: "person" },
        ],
      },
      {
        addr: Address.make("docs"),
        guide: "├─",
        waiting: 0,
        working: 1,
        done: 1,
        latest: 4_000,
        since: 4_000,
        starts: [
          { run: "r3", at: 2_000, phase: "done" },
          { run: "r4", at: 4_000, phase: "idle" },
        ],
      },
      { addr: Address.make("hall"), guide: "└─", waiting: 0, working: 0, done: 0, latest: null, since: null, starts: [] },
    ]);
  });
});
