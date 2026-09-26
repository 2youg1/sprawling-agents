// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import type { RunBelief } from "./belief/shape";
import type { Doing } from "./doing";
import { FIRST, drawsCalls, resultsOf } from "./results";
import { RunId, Seq, TimeMs } from "../wire";

// The fixture city of the contract: two hundred runs, forty of each of
// the five postures a run can be found in.
const RUNS = 200;
const POSTURES: readonly Doing[] = [
  { kind: "waiting" },
  { kind: "frozen", completion: "limit" },
  { kind: "frozen", completion: "done" },
  { kind: "thinking" },
  { kind: "frozen", completion: "cancelled" },
];
// What triaging the fixture city into the three groups may spend: a
// tenth of a 16 ms frame, so the rows the page draws are the frame's
// only cost that grows with what is shown rather than with the city.
const BUDGET_US = 1600;

function runAt(index: number): RunBelief {
  return {
    run: RunId.make(`00000000-0000-4000-8000-${index.toString(16).padStart(12, "0")}`),
    addr: null,
    started: TimeMs.make(index),
    task: `task ${String(index)}`,
    lastSeq: Seq.make(1),
    doing: POSTURES[index % POSTURES.length] ?? { kind: "unknown" },
    local: false,
    saying: "",
    thinking: "",
  };
}

const CITY = Array.from({ length: RUNS }, (_, index) => runAt(index));

describe("results-only mode", () => {
  test("the room mounts no calls in results mode", () => {
    expect([drawsCalls("whole"), drawsCalls("results")]).toEqual([true, false]);
  });

  test("the city draws only the newest first N of waiting, failed and done", () => {
    const groups = resultsOf(CITY, FIRST);
    expect(
      groups.map((group) => [group.outcome, group.total, group.first.map((run) => run.task)]),
    ).toEqual([
      ["waiting", 40, ["task 195", "task 190", "task 185", "task 180", "task 175"]],
      ["failed", 40, ["task 196", "task 191", "task 186", "task 181", "task 176"]],
      ["done", 40, ["task 197", "task 192", "task 187", "task 182", "task 177"]],
    ]);
  });

  test("triaging the two-hundred-run city stays inside a tenth of a frame", () => {
    const rounds = 200;
    const start = performance.now();
    for (let each = 0; each < rounds; each += 1) resultsOf(CITY, FIRST);
    const perCityUs = ((performance.now() - start) * 1000) / rounds;
    console.log(`city_results runs=${String(RUNS)} first=${String(FIRST)} per_city_us=${perCityUs.toFixed(1)}`);
    expect(perCityUs).toBeLessThanOrEqual(BUDGET_US);
  });
});
