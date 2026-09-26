// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import type { BoardRun } from "./lineage";
import { rowsOf, windowOf } from "./lineage";

function run(id: string, addr: string, started: number, doing: BoardRun["doing"]): BoardRun {
  return { run: id, addr, task: null, started, ended: null, doing };
}

const THINKING = { kind: "thinking" } as const;
const WAITING = { kind: "waiting" } as const;

describe("lineage", () => {
  // The first screen of a long board is the part a person has to act
  // on: a waiting run leads its room, its room leads the building, and
  // the building leads the city, whatever the names and ages say.
  test("a run waiting for the person leads at every level", () => {
    const runs = [
      run("r1", "alpha/a", 300, THINKING),
      run("r2", "zeta/b", 100, THINKING),
      run("r3", "zeta/z", 200, THINKING),
      run("r4", "zeta/z", 100, WAITING),
    ];
    expect(rowsOf(runs, new Set()).map((row) => row.key)).toEqual(["zeta", "zeta/z", "r4", "r3", "zeta/b", "r2", "alpha", "alpha/a", "r1"]);
  });

  test("a folded building keeps its row and hides what is under it", () => {
    const runs = [run("r1", "alpha/a", 1, THINKING), run("r2", "beta/b", 1, THINKING)];
    expect(rowsOf(runs, new Set(["alpha"])).map((row) => row.key)).toEqual(["alpha", "beta", "beta/b", "r2"]);
  });

  test("a thousand rows draw only the viewport and its margin", () => {
    expect(windowOf(1200, 2800, 560, 28, 8)).toEqual({ from: 92, to: 128 });
    expect(windowOf(1200, 0, 560, 0, 8)).toEqual({ from: 0, to: 8 });
  });
});
