// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";
import { get } from "svelte/store";

import { createBelief } from "../belief";
import type { CityAnswer, RunSummary } from "../../wire";
import { RunId, Seq } from "../../wire";

const RUNS = 10_000;
const DELTAS = 2_000;
// The budget one token increment may spend folding into a city of
// `RUNS` runs. A fold that copies the run table is linear in the city,
// and at this size that is a frame's worth of work per token.
const BUDGET_US = 20;

function runId(index: number): RunId {
  return RunId.make(`00000000-0000-4000-8000-${index.toString(16).padStart(12, "0")}`);
}

function listed(run: RunId): RunSummary {
  return {
    run,
    addr: null,
    started: null,
    last_seq: Seq.make(1),
    last_kind: "run_started",
    frozen: false,
    who: "hall/mayor",
  };
}

describe("fold cost", () => {
  test("one delta folds in constant time however many runs the city holds", () => {
    const store = createBelief(() => 0);
    const runs = Array.from({ length: RUNS }, (_, index) => listed(runId(index)));
    const city: CityAnswer = { active: RUNS, buildings: [], frozen: 0, halted: [], pursuits: [], runs };
    store.adoptCity(city);
    const target = runId(RUNS / 2);
    const start = performance.now();
    for (let each = 0; each < DELTAS; each += 1) {
      store.say({ run: target, increment: { said: "x" } });
    }
    const perDeltaUs = ((performance.now() - start) * 1000) / DELTAS;
    expect(get(store.belief).runs[target]?.saying.length).toBe(DELTAS);
    expect(perDeltaUs).toBeLessThanOrEqual(BUDGET_US);
  });
});
