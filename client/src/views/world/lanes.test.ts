// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { GitOid } from "../../wire";
import { graphOf } from "./lanes";
import type { Commit } from "./lanes";

const oid = (name: string): GitOid => GitOid.make(name.padEnd(40, "0"));
const commit = (name: string, ...parents: string[]): Commit => ({ oid: oid(name), parents: parents.map(oid) });

describe("the swimlane a page of commits draws", () => {
  test("a straight history is one lane, each commit joined to the next", () => {
    const graph = graphOf([commit("c", "b"), commit("b", "a"), commit("a")]);
    expect(graph).toEqual({
      lanes: 1,
      rows: [
        { lane: 0, lines: [{ from: 0, to: 0, part: "out", owner: 0 }] },
        { lane: 0, lines: [{ from: 0, to: 0, part: "in", owner: 0 }, { from: 0, to: 0, part: "out", owner: 1 }] },
        { lane: 0, lines: [{ from: 0, to: 0, part: "in", owner: 1 }] },
      ],
    });
  });

  test("a merge leaves its node sideways and the branch rejoins where its parent is", () => {
    // e merges f into b; f and b both come from a.
    const graph = graphOf([commit("e", "b", "f"), commit("f", "a"), commit("b", "a"), commit("a")]);
    expect(graph.lanes).toBe(2);
    expect(graph.rows.map((row) => row.lane)).toEqual([0, 1, 0, 0]);
    // The merge row draws its first parent down its own lane and its
    // second out to lane 1.
    expect(graph.rows[0]?.lines).toEqual([
      { from: 0, to: 0, part: "out", owner: 0 },
      { from: 0, to: 1, part: "out", owner: 0 },
    ]);
    // Where both lanes wait for `a`, both lines run into its node.
    expect(graph.rows[3]?.lines).toEqual([
      { from: 0, to: 0, part: "in", owner: 2 },
      { from: 1, to: 0, part: "in", owner: 1 },
    ]);
  });

  test("a parent the page does not hold keeps its lane running to the foot", () => {
    const graph = graphOf([commit("b", "d"), commit("c", "9")]);
    expect(graph.rows[1]).toEqual({
      lane: 1,
      lines: [
        { from: 0, to: 0, part: "through", owner: 0 },
        { from: 1, to: 1, part: "out", owner: 1 },
      ],
    });
  });

  test("a commit whose parents the wire did not carry ends its lane", () => {
    const graph = graphOf([{ oid: oid("b"), parents: null }, commit("a")]);
    expect(graph.rows).toEqual([
      { lane: 0, lines: [] },
      { lane: 0, lines: [] },
    ]);
  });
});
