// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// A person on a subscription login or a local model is never billed per
// call, so a city that has run a dozen times answers with a zero total.
// The server counts those calls in `unpriced`; the page has to read that
// count, or it tells the person that nothing happened.

import { describe, expect, test } from "bun:test";

import { costReading, runSpend } from "./pricing";
import { RunId, UsdMicros, type CostAnswer } from "../wire";

function answer(total: number, calls: number, tokens: number): CostAnswer {
  return {
    by_actor: [],
    by_run: [],
    by_segment: [],
    by_skill: [],
    by_tool: [],
    total: UsdMicros.make(total),
    unpriced: { calls, tokens },
  };
}

describe("a zero total is read by whether any call went unpriced", () => {
  test("calls no provider priced are not an idle city", () => {
    expect(costReading(answer(0, 13, 48_000))).toEqual({ kind: "unpriced" });
  });

  test("a city with no calls at all is idle", () => {
    expect(costReading(answer(0, 0, 0))).toEqual({ kind: "idle" });
  });

  test("a priced total is priced even beside unpriced calls", () => {
    expect(costReading(answer(700, 2, 1_542))).toEqual({ kind: "priced" });
  });
});

describe("the facts row reads a run's spend only when a run is moving", () => {
  test("no run moving says nothing, even when the cost answer is unavailable", () => {
    expect(runSpend({ kind: "unavailable", query: "cost" }, null)).toEqual({ kind: "none" });
  });

  test("a moving run with an unreadable cost answer says so", () => {
    expect(runSpend({ kind: "unavailable", query: "cost" }, RunId.make("00000000-0000-4000-8000-000000000001"))).toEqual({ kind: "unreadable" });
  });
});
