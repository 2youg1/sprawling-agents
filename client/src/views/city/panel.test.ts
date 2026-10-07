// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import type { RunBelief } from "../../core/belief";
import type { Doing } from "../../core/doing";
import { Address, NodeId, RunId, Seq, Tokens, UsdMicros } from "../../wire";
import type { BuildingProgress, CityAnswer } from "../../wire";
import { panelLookOf } from "./panel";

const IDLE = { unplanned: { budget: { tokens: Tokens.make(0), usd: UsdMicros.make(0) }, steps: 0 } };

function building(addr: string, more: Partial<BuildingProgress> = {}): BuildingProgress {
  return { addr: Address.make(addr), blocked: [], problems: [], progress: IDLE, ready: 0, ...more };
}

function city(buildings: readonly BuildingProgress[]): CityAnswer {
  return { active: 0, frozen: 0, halted: [], pursuits: [], runs: [], buildings: [...buildings] };
}

function run(index: number, doing: Doing): RunBelief {
  return {
    run: RunId.make(`00000000-0000-4000-8000-${index.toString(16).padStart(12, "0")}`),
    addr: Address.make("docs/zh"),
    started: null,
    task: `task ${String(index)}`,
    goal: null,
    lastSeq: Seq.make(1),
    doing,
    model: null,
    pr: null,
    ask: null,
    local: false,
    saying: "",
    thinking: "",
  };
}

describe("the picked building's panel", () => {
  // A building with no plan has no denominator: it carries no progress
  // and nothing "ready to start", only what is stuck, rather than a
  // 0/0 that reads as a plan nobody began.
  test("progress and readiness are said only of a planned building, stuck lines always", () => {
    const blocked = [{ source: NodeId.make("docs/zh"), line: "waiting for a glossary decision", waiting: 1 }];
    const unplanned = panelLookOf(
      { addr: Address.make("docs"), city: city([building("docs", { blocked, ready: 2 })]), held: [], lang: "en" },
      () => undefined,
    );
    expect(unplanned.badges).toEqual([{ key: "stuck", text: "1 stuck", weight: "alert" }]);

    const progress = { planned: { blocked: 1, blocked_ppb: 125_000_000, done: 3, done_ppb: 375_000_000, total: 8 } };
    const planned = panelLookOf(
      { addr: Address.make("docs"), city: city([building("docs", { blocked, progress, ready: 2 })]), held: [], lang: "en" },
      () => undefined,
    );
    expect(planned.badges).toEqual([
      { key: "progress", text: "3/8", weight: "quiet" },
      { key: "ready", text: "2 ready to start", weight: "live" },
      { key: "stuck", text: "1 stuck", weight: "alert" },
    ]);
  });

  test("the panel lists the building's eight newest runs, newest first, and closes through its seat", () => {
    const held = Array.from({ length: 10 }, (_, n) => run(n, n === 9 ? { kind: "waiting" } : { kind: "frozen", completion: "done" }));
    let closed = 0;
    const look = panelLookOf({ addr: Address.make("docs"), city: city([building("docs")]), held, lang: "en" }, () => {
      closed += 1;
    });
    expect(look.runs.rows.map((row) => [row.title, row.moving])).toEqual([
      ["task 9", true],
      ["task 8", false],
      ["task 7", false],
      ["task 6", false],
      ["task 5", false],
      ["task 4", false],
      ["task 3", false],
      ["task 2", false],
    ]);
    look.close.onPress();
    expect(closed).toBe(1);
  });
});
