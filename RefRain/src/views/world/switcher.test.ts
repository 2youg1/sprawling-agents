// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { Address, RunId, Seq, TimeMs } from "../../wire";
import type { SessionNode } from "../../core/session_tree";
import type { Stretch } from "../../core/stretches";
import { answer, switchItems } from "./switcher";

const node = (key: string, children: boolean): SessionNode => {
  const room = Address.make(key);
  const stretch: Stretch = {
    room,
    line: {
      began: Seq.make(1),
      start: { dispatched: { by: null } },
      runs: 0,
      last: Seq.make(1),
      at: TimeMs.make(1),
    },
    current: true,
    runs: [],
  };
  return { stretch, key, depth: 0, children, delegated: false };
};

describe("the session switcher keys", () => {
  const nodes = [node("a", true), node("b", false)];
  test("up and down stay inside, left and right only fold, and Enter alone opens", () => {
    const start = { cursor: 0, collapsed: new Set<string>() };
    expect(answer(nodes, start, "up")[0].cursor).toBe(0);
    expect(answer(nodes, start, "down")[0].cursor).toBe(1);
    expect(answer(nodes, start, "down")[1]).toBe(false);
    const folded = answer(nodes, start, "left");
    expect([...folded[0].collapsed]).toEqual(["a"]);
    expect(folded[1]).toBe(false);
    expect([...answer(nodes, folded[0], "right")[0].collapsed]).toEqual([]);
    expect(answer(nodes, start, "enter")[1]).toBe(true);
    expect(answer([], start, "enter")[1]).toBe(false);
  });

  test("composition and other keys change nothing", () => {
    const start = { cursor: 1, collapsed: new Set(["a"]) };
    expect(answer(nodes, start, "composing")).toEqual([start, false]);
    expect(answer(nodes, start, "other")).toEqual([start, false]);
  });
});

describe("the switcher reads the shared tree", () => {
  test("a current row navigates and a past row carries its tail", () => {
    const room = Address.make("lab/room");
    const stretch: Stretch = {
      room,
      line: {
        began: Seq.make(3),
        start: { dispatched: { by: null } },
        runs: 0,
        last: Seq.make(4),
        at: TimeMs.make(1),
        tail: { run: RunId.make("00000000-0000-4000-8000-000000000001"), at_seq: Seq.make(4) },
      },
      current: false,
      runs: [],
    };
    const rows = switchItems([stretch], "", new Set(), "en", 61_000);
    expect(rows[0]?.origin).toEqual(stretch.line.tail);
    expect(rows[0]?.to).toEqual({ kind: "talk", address: room, session: Seq.make(3) });
  });
});
