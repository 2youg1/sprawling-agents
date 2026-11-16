// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { Address, RunId, Seq, TimeMs } from "../wire";
import type { SessionLine, SessionRef } from "../wire";
import { defaultCollapsed, sessionKey, sessionTree } from "./session_tree";
import type { Stretch } from "./stretches";
import { tailOf } from "./stretches";

const MOTHER = RunId.make("00000000-0000-4000-8000-000000000001");

function line(
  room: Address,
  began: number,
  parent: SessionRef | undefined,
  delegated: boolean,
): SessionLine {
  return {
    began: Seq.make(began),
    start: delegated
      ? { dispatched: { by: null } }
      : {
          opened: {
            carry: "nothing",
            from: parent === undefined ? null : { run: MOTHER, at_seq: Seq.make(1) },
          },
        },
    runs: 1,
    last: Seq.make(began + 1),
    at: TimeMs.make(began),
    parent,
    branched_from: delegated
      ? null
      : parent === undefined
        ? null
        : { run: MOTHER, at_seq: Seq.make(1) },
    tail: { run: MOTHER, at_seq: Seq.make(began + 1) },
  };
}

function stretch(room: string, began: number, parent?: SessionRef, delegated = false): Stretch {
  const addr = Address.make(room);
  return { room: addr, line: line(addr, began, parent, delegated), current: false, runs: [] };
}

const A = stretch("hall/mayor", 1);
const B = stretch("lab/worker", 2, { room: A.room, began: A.line.began }, true);
const C = stretch("lab/review", 3, { room: B.room, began: B.line.began });
const ALL = [C, B, A];

describe("one session tree", () => {
  test("delegated children collapse by default while branch children stay visible", () => {
    const collapsed = defaultCollapsed(ALL);
    expect([...collapsed]).toEqual([sessionKey(A)]);
    expect(sessionTree(ALL, collapsed).map((node) => [node.key, node.depth])).toEqual([
      [sessionKey(A), 0],
    ]);
    expect(
      sessionTree(ALL, new Set()).map((node) => [node.key, node.depth, node.delegated]),
    ).toEqual([
      [sessionKey(A), 0, false],
      [sessionKey(B), 1, true],
      [sessionKey(C), 2, false],
    ]);
  });

  test("a missing parent is a root and a cycle cannot recurse forever", () => {
    const missing = stretch("lab/missing", 4, {
      room: Address.make("lab/unknown"),
      began: Seq.make(9),
    });
    const left = stretch("lab/left", 5);
    const right = stretch("lab/right", 6, { room: left.room, began: left.line.began });
    const cyclic: Stretch = {
      ...left,
      line: { ...left.line, parent: { room: right.room, began: right.line.began } },
    };
    const nodes = sessionTree([missing, right, cyclic], new Set());
    expect(nodes.map((node) => node.key)).toHaveLength(3);
    expect(nodes[0]?.depth).toBe(0);
  });

  test("a tail comes from the sessions answer even when no run is held", () => {
    expect(tailOf(A)).toEqual(A.line.tail ?? null);
  });
});
