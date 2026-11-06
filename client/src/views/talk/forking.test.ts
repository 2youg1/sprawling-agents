// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import type { View } from "../../core/route";
import type { RoundsAnswer, SessionLine, Turn } from "../../wire";
import { Address, RunId, Seq, TimeMs } from "../../wire";
import { motherName, originalOf, roundsThrough } from "./forking";

describe("how the divider names the mother run", () => {
  test("by the first sentence of its task, in either script", () => {
    expect(motherName("Measure the latency. Then write it down.")).toBe("Measure the latency.");
    expect(motherName("测五次延迟。然后记下来。")).toBe("测五次延迟。");
    expect(motherName("  fix the parser\nand its tests")).toBe("fix the parser");
  });

  test("a task with no sentence end is named whole", () => {
    expect(motherName("fix the parser")).toBe("fix the parser");
  });

  test("a blank task names nothing, so the divider falls back to its generic label", () => {
    expect([motherName(""), motherName("  \n  ")]).toEqual([null, null]);
  });
});

describe("where the way back from a branch leads", () => {
  const MOTHER = RunId.make("0199c0de-1a2b-4c3d-8e4f-5a6b7c8d9e0f");
  const ROOM = Address.make("mayor");
  const line = (began: number): SessionLine => ({
    at: TimeMs.make(began),
    began: Seq.make(began),
    last: Seq.make(began + 9),
    runs: 1,
    start: { opened: { carry: "nothing" } },
  });
  // Newest first, as the city answers them: the branch opened at 40,
  // the original at 10, an older stretch at 1.
  const LINES = [line(40), line(10), line(1)];

  test("to the stretch the mother run lies in, read in main as an earlier session", () => {
    expect(originalOf(MOTHER, { addr: ROOM, lastSeq: Seq.make(25) }, LINES)).toEqual({
      kind: "talk",
      address: ROOM,
      session: Seq.make(10),
    });
  });

  test("a run whose last line opens a stretch belongs to that stretch", () => {
    expect(originalOf(MOTHER, { addr: ROOM, lastSeq: Seq.make(10) }, LINES)).toEqual({
      kind: "talk",
      address: ROOM,
      session: Seq.make(10),
    });
  });

  test("to the run's own page while the run, its room or its stretch is unknown", () => {
    const page: View = { kind: "run", run: MOTHER };
    expect([
      originalOf(MOTHER, undefined, LINES),
      originalOf(MOTHER, { addr: null, lastSeq: Seq.make(25) }, LINES),
      originalOf(MOTHER, { addr: ROOM, lastSeq: Seq.make(25) }, []),
      originalOf(MOTHER, { addr: ROOM, lastSeq: Seq.make(0) }, LINES),
    ]).toEqual([page, page, page, page]);
  });
});

test("a branch exposes only inherited turns, calls and notes without changing the mother", () => {
  const turn: Turn = {
    calls: [12, 18].map((at) => ({
      at: Seq.make(at),
      called: TimeMs.make(at),
      tool: "read",
      outcome: "answered",
      timing: "measured",
    })),
    notes: [14, 19].map((at) => ({ discarded: { at: Seq.make(at), count: 1 } })),
    number: 1,
    opened: Seq.make(10),
    t: TimeMs.make(10),
    timing: "measured",
    said: "inherited",
  };
  const mother: RoundsAnswer = {
    run: RunId.make("0199c0de-1a2b-4c3d-8e4f-5a6b7c8d9e0f"),
    turns: [turn, { ...turn, opened: Seq.make(20), said: "later" }],
  };
  const inherited = roundsThrough(mother, Seq.make(14));
  expect(inherited.turns.map((each) => each.said)).toEqual(["inherited"]);
  expect(inherited.turns[0]?.calls.map((call) => call.at)).toEqual([Seq.make(12)]);
  expect(inherited.turns[0]?.notes).toEqual([{ discarded: { at: Seq.make(14), count: 1 } }]);
  expect(roundsThrough(mother, Seq.make(9)).turns).toEqual([]);
  expect(roundsThrough(mother, null)).toBe(mother);
  expect(mother.turns).toHaveLength(2);
  expect(mother.turns[0]?.calls).toHaveLength(2);
});
