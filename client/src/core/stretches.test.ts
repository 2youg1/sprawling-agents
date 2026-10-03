// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { Address, RunId, Seq, Tag, TimeMs } from "../wire";
import type { SessionLine, SessionsAnswer } from "../wire";
import type { RunBelief } from "./belief";
import { unseen } from "./belief/fold";
import { MAYOR } from "./route";
import { grouped, lineIn, pinningOf, stretchesOf, tailOf } from "./stretches";
import type { Stretch } from "./stretches";
import { PIN } from "./tags";

const LAB = Address.make("lab/room1");
const run = (n: number, room: Address, last: number): RunBelief => ({
  ...unseen(RunId.make(`00000000-0000-4000-8000-${n.toString(16).padStart(12, "0")}`), Seq.make(last)),
  addr: room,
});
const line = (began: number, at: number): SessionLine => ({
  began: Seq.make(began),
  last: Seq.make(began + 1),
  at: TimeMs.make(at),
  runs: 1,
  start: { dispatched: { by: null } },
});
// Newest first, as the city answers.
const answer = (room: Address, ...lines: SessionLine[]): SessionsAnswer => ({ room, sessions: lines, earlier: 0 });

describe("the sessions the pane lists", () => {
  const second = run(2, LAB, 12);
  const runs = [run(1, LAB, 4), second, run(3, LAB, 25), run(4, MAYOR, 31)];
  const all = stretchesOf(
    [answer(LAB, line(20, 300), line(10, 200), line(1, 100)), answer(MAYOR, line(30, 250))],
    (room) => runs.filter((each) => each.addr === room),
  );

  test("each stretch holds the runs whose last line falls in it, and only the newest is current", () => {
    expect(all.map((each) => [each.room, each.line.began, each.current, each.runs.map((held) => held.lastSeq)])).toEqual([
      [LAB, Seq.make(20), true, [Seq.make(25)]],
      [MAYOR, Seq.make(30), true, [Seq.make(31)]],
      [LAB, Seq.make(10), false, [Seq.make(12)]],
      [LAB, Seq.make(1), false, [Seq.make(4)]],
    ]);
  });

  test("the route's stretch is the named one, or the current one when it names none", () => {
    const lab = answer(LAB, line(20, 300), line(10, 200));
    expect([lineIn(lab, undefined), lineIn(lab, Seq.make(10)), lineIn(lab, Seq.make(7)), lineIn(undefined, undefined)]).toEqual([
      line(20, 300),
      line(10, 200),
      null,
      null,
    ]);
  });

  test("the Mayor's current session is pinned by being current; any other by its tag", () => {
    const mayor = all.find((each) => each.room === MAYOR);
    const past = all.find((each) => !each.current);
    expect([mayor && pinningOf(mayor, []), past && pinningOf(past, [PIN]), past && pinningOf(past, [])]).toEqual([
      "mayor",
      "tagged",
      "none",
    ]);
  });

  test("the pinned come first, then each building newest first, and a filter keeps only its tag", () => {
    const bug = Tag.make("bug");
    const tagsFor = (each: Stretch): Tag[] => (each.line.began === Seq.make(1) ? [PIN, bug] : each.line.began === Seq.make(10) ? [bug] : []);
    const shape = (filter: Tag | null): unknown =>
      grouped(all, tagsFor, filter).map((group) => [group.kind === "pinned" ? "pinned" : group.building, group.rows.map((each) => each.line.began)]);
    expect(shape(null)).toEqual([
      ["pinned", [Seq.make(30), Seq.make(1)]],
      [Address.make("lab"), [Seq.make(20), Seq.make(10)]],
    ]);
    expect(shape(bug)).toEqual([
      ["pinned", [Seq.make(1)]],
      [Address.make("lab"), [Seq.make(10)]],
    ]);
  });

  test("going on from a stretch branches at its last run's last line", () => {
    const past = all.find((each) => each.line.began === Seq.make(10));
    const lone: Stretch = { room: LAB, line: line(40, 400), current: true, runs: [] };
    expect([past && tailOf(past), tailOf(lone)]).toEqual([{ run: second.run, at_seq: Seq.make(12) }, null]);
  });
});
