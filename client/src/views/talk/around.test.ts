// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { Seq, TimeMs, type Turn } from "../../wire";
import { turnsAround } from "./around";

function turn(opened: number): Turn {
  return { calls: [], notes: [], number: opened, opened: Seq.make(opened), t: TimeMs.make(opened), timing: "measured" };
}

const FIVE = [10, 20, 30, 40, 50].map(turn);

describe("a letter draws the turns around the line that sent it (client D91)", () => {
  test("the turn holding the line, with one turn either side", () => {
    expect(turnsAround(FIVE, Seq.make(34))).toEqual({ from: 1, to: 4, cut: true });
  });

  test("a line in the first or the last turn keeps the window inside the session", () => {
    expect([turnsAround(FIVE, Seq.make(12)), turnsAround(FIVE, Seq.make(99))]).toEqual([
      { from: 0, to: 2, cut: true },
      { from: 3, to: 5, cut: true },
    ]);
  });

  test("no line, or one before the first turn, draws the whole session", () => {
    expect([turnsAround(FIVE, null), turnsAround(FIVE, Seq.make(3))]).toEqual([
      { from: 0, to: 5, cut: false },
      { from: 0, to: 5, cut: false },
    ]);
  });

  test("a short session drawn whole says nothing was left out", () => {
    expect(turnsAround(FIVE.slice(0, 3), Seq.make(20))).toEqual({ from: 0, to: 3, cut: false });
  });
});
