// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the time lens decides before anything is drawn: which share
// each turn belongs to, how many boxes a lane may hold, and which rows
// of a long call list exist in the page at all. Where the boxes land is
// measured by `cargo xtask render`, not here.

import { describe, expect, test } from "bun:test";

import { Seq, TimeMs, type Call, type Note, type Turn } from "../../wire";
import { OVERSCAN, callsOf, columnsOf, stretchesOf, windowOf } from "./lanes";

function call(n: number, called = 0, answered: number | null = null): Call {
  return {
    tool: "read",
    subject: `f${String(n)}`,
    arguments: null,
    outcome: answered === null ? "waiting" : "answered",
    at: Seq.make(n),
    output: null,
    called: TimeMs.make(called),
    answered: answered === null ? null : TimeMs.make(answered),
  };
}

function turn(number: number, t: number, calls: readonly Call[], notes: readonly Note[] = []): Turn {
  return { calls, notes, number, opened: Seq.make(number), t: TimeMs.make(t) };
}

describe("the time lens", () => {
  test("a turn is held by the person when it waited, by tools when it called, by the model otherwise", () => {
    const turns = [
      turn(1, 0, []),
      turn(2, 10, [call(1, 10, 30)]),
      turn(3, 30, [], [{ waiting: { at: Seq.make(9) } }]),
      turn(4, 45, []),
    ];
    expect(stretchesOf(turns, 60, null)).toEqual([
      { share: "model", turn: 1, from: 0, to: 10 },
      { share: "tool", turn: 2, from: 10, to: 30 },
      { share: "person", turn: 3, from: 30, to: 45 },
      { share: "model", turn: 4, from: 45, to: 60 },
    ]);
    expect(stretchesOf(turns, 60, "tool").at(-1)?.share).toBe("tool");
  });

  // Two calls overlap between 14 and 22; the model spoke before the
  // first and after the last answer, and those are its own stretches.
  test("a turn that called tools is cut where its calls were measured", () => {
    const turns = [turn(1, 10, [call(1, 14, 20), call(2, 16, 22)]), turn(2, 30, [call(3, 31)])];
    expect(stretchesOf(turns, 40, null)).toEqual([
      { share: "model", turn: 1, from: 10, to: 14 },
      { share: "tool", turn: 1, from: 14, to: 22 },
      { share: "model", turn: 1, from: 22, to: 30 },
      { share: "model", turn: 2, from: 30, to: 31 },
      { share: "tool", turn: 2, from: 31, to: 40 },
    ]);
  });

  // Ten thousand turns across four hundred columns: a lane is at most
  // one box per column, so the page holds a bounded number of boxes
  // however long the run went on.
  test("a lane never holds more boxes than it has columns", () => {
    const turns = Array.from({ length: 10_000 }, (_, n) => turn(n + 1, n * 1000, n % 3 === 0 ? [call(n)] : []));
    const lane = columnsOf(stretchesOf(turns, 10_000_000, null), 0, 10_000_000, 400);
    expect(lane.length).toBeLessThanOrEqual(400);
    expect(lane[0]?.first).toBe(0);
    expect(lane.at(-1)?.end).toBe(400);
  });

  test("a call list of ten thousand rows draws only the rows in view", () => {
    const turns = Array.from({ length: 100 }, (_, n) => turn(n + 1, n, Array.from({ length: 100 }, (_, c) => call(c))));
    const total = callsOf(turns).length;
    expect(total).toBe(10_000);
    const rows = windowOf(total, 28, 5_000 * 28, 560);
    expect(rows).toEqual({ first: 5_000 - OVERSCAN, end: 5_020 + OVERSCAN });
    expect(windowOf(total, 28, 0, 560)).toEqual({ first: 0, end: 20 + OVERSCAN });
  });
});
