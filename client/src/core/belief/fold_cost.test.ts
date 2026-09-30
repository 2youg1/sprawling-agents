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
// A small city, the baseline a large one is measured against in the
// same run: a bound in microseconds would be a constant tuned to one
// machine, while a ratio holds on any machine that runs both. A fold
// that copies the run table is linear in the city, which puts the
// large city a hundred times over the small one; constant time puts
// them within timer noise, and `SLOWER` is that noise with room.
const BASELINE_RUNS = 100;
const SLOWER = 5;
// One animation frame of stream. A delta into a run the table holds
// wakes only that run's readers, so a subscriber that reads the whole
// run table hears the city's answer and no token after it.
const FRAME_DELTAS = 50;
const FRAMES = 40;

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
    const [small, large] = [BASELINE_RUNS, RUNS].map((size) => {
      const store = createBelief(() => 0);
      store.adoptCity(adoptedCity(size));
      const target = runId(size / 2);
      const start = performance.now();
      for (let each = 0; each < DELTAS; each += 1) {
        store.say({ run: target, increment: { said: "x" } });
      }
      const perDeltaUs = ((performance.now() - start) * 1000) / DELTAS;
      expect(get(store.belief).runs[target]?.saying.length).toBe(DELTAS);
      return perDeltaUs;
    });
    expect(large).toBeLessThanOrEqual((small ?? 0) * SLOWER);
  });

  // Prints the reading per city size for `tools/xtask/budgets.toml`'s
  // `client_fold` row, and holds the count every machine agrees on: the
  // subscriber reads the table once when it subscribes, empty, and once
  // for the city's answer, and never for a token.
  test("one frame of deltas tells a table-reading subscriber once", () => {
    for (const size of [100, 1_000, RUNS]) {
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
      expect(read).toBe(size);
    }
  });
});
