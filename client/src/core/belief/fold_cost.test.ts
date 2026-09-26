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
// One animation frame of stream, and what folding it may spend with a
// subscriber that reads the whole run table: a quarter of a 16 ms frame.
// A delta into a run the table holds wakes only that run's readers, so
// the subscriber hears the city's answer and no token after it.
const FRAME_DELTAS = 50;
const FRAMES = 40;
const FRAME_BUDGET_US = 4000;

function runId(index: number): RunId {
  return RunId.make(`00000000-0000-4000-8000-${index.toString(16).padStart(12, "0")}`);
}

function adoptedCity(size: number): CityAnswer {
  const runs = Array.from({ length: size }, (_, index) => listed(runId(index)));
  return { active: size, buildings: [], frozen: 0, halted: [], pursuits: [], runs };
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
    store.adoptCity(adoptedCity(RUNS));
    const target = runId(RUNS / 2);
    const start = performance.now();
    for (let each = 0; each < DELTAS; each += 1) {
      store.say({ run: target, increment: { said: "x" } });
    }
    const perDeltaUs = ((performance.now() - start) * 1000) / DELTAS;
    expect(get(store.belief).runs[target]?.saying.length).toBe(DELTAS);
    expect(perDeltaUs).toBeLessThanOrEqual(BUDGET_US);
  });

  // Prints the reading per city size for `xtask/budgets.toml`'s
  // `client_fold` row, and holds the largest to the frame budget.
  test("one frame of deltas tells a table-reading subscriber once", () => {
    const readings = [100, 1_000, RUNS].map((size) => {
      const store = createBelief(() => 0);
      let read = 0;
      const stop = store.belief.subscribe((belief) => {
        read += Object.keys(belief.runs).length;
      });
      store.adoptCity(adoptedCity(size));
      const target = runId(size / 2);
      const start = performance.now();
      for (let frame = 0; frame < FRAMES; frame += 1) {
        store.batch(() => {
          for (let each = 0; each < FRAME_DELTAS; each += 1) {
            store.say({ run: target, increment: { thought: "x" } });
          }
        });
      }
      const perFrameUs = ((performance.now() - start) * 1000) / FRAMES;
      stop();
      console.log(`client_fold R=${String(size)} per-frame-us=${perFrameUs.toFixed(1)} read=${String(read)}`);
      return perFrameUs;
    });
    expect(readings.at(-1)).toBeLessThanOrEqual(FRAME_BUDGET_US);
  });
});
