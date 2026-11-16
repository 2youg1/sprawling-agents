// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { WORKBENCH, moved, readWorkbench, resized, spelledWorkbench, widest } from "./workbench";
import type { Workbench } from "./workbench";

const spans = (bench: Workbench): readonly number[] => bench.map((column) => column.span);
const panes = (bench: Workbench): readonly string[] => bench.map((column) => column.pane);

describe("a divider moves one pane's edge and no other", () => {
  test("the pane after the divider takes what the pane before it gave up", () => {
    expect(spans(resized(WORKBENCH, 0, 5))).toEqual([5, 4, 3]);
    expect(spans(resized(WORKBENCH, 1, 7))).toEqual([3, 7, 2]);
  });

  test("no pane is pushed under two columns from either side", () => {
    expect(spans(resized(WORKBENCH, 0, 0))).toEqual([2, 7, 3]);
    expect(spans(resized(WORKBENCH, 0, 11))).toEqual([7, 2, 3]);
    expect(widest(WORKBENCH, 0)).toBe(7);
    expect(widest(WORKBENCH, 1)).toBe(7);
  });

  test("a width between two column lines is not a width", () => {
    expect(spans(resized(WORKBENCH, 0, 4.6))).toEqual([5, 4, 3]);
  });
});

describe("a pane moves past its neighbour with its width", () => {
  test("moving right swaps it with the pane to its right", () => {
    const bench = moved(WORKBENCH, "sessions", "right");
    expect(panes(bench)).toEqual(["session", "sessions", "commits"]);
    expect(spans(bench)).toEqual([6, 3, 3]);
  });

  test("a pane at the edge stays where it is", () => {
    expect(moved(WORKBENCH, "sessions", "left")).toEqual(WORKBENCH);
    expect(moved(WORKBENCH, "commits", "right")).toEqual(WORKBENCH);
  });
});

describe("the row a browser keeps", () => {
  test("a spelled arrangement reads back as itself", () => {
    const bench = resized(moved(WORKBENCH, "commits", "left"), 1, 3);
    expect(readWorkbench(spelledWorkbench(bench))).toEqual(bench);
  });

  test("a row this build cannot read is the arrangement it ships with", () => {
    for (const raw of [
      null,
      "",
      "sessions:3 session:5",
      "sessions:3 sessions:5 commits:4",
      "sessions:1 session:7 commits:4",
      "sessions:3 session:5 commits:5",
      "sessions:3.5 session:4.5 commits:4",
      "sessions:x session:5 commits:4",
    ]) {
      expect(readWorkbench(raw)).toEqual(WORKBENCH);
    }
  });
});
