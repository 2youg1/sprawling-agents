// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// A person on a subscription login or a local model pays nothing per
// call, so a city that has run a dozen times answers with a zero total
// and a by-run cut full of zero rows. The page used to read that answer
// as "no run has spent anything yet", which states that nothing
// happened; the test below holds the line between the two situations.

import { describe, expect, test } from "bun:test";

import { costReading } from "./cost";
import { UsdMicros, type CostAnswer } from "../wire";

function answer(total: number, runs: readonly string[]): CostAnswer {
  const zero = UsdMicros.make(0);
  return {
    by_actor: [],
    by_run: runs.map((run) => [run, zero] as const),
    by_segment: [],
    by_skill: [],
    by_tool: [],
    total: UsdMicros.make(total),
  };
}

describe("a zero total is read by whether anything ran", () => {
  test("runs that no provider priced are not an idle city", () => {
    expect(costReading(answer(0, ["r1", "r2", "r3"]))).toEqual({ kind: "unpriced", runs: 3 });
  });

  test("a city with no runs is idle", () => {
    expect(costReading(answer(0, []))).toEqual({ kind: "idle" });
  });
});
