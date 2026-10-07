// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// A tree row's wiring: only a folder discloses anything, so only a folder
// states whether it stands open.

import { describe, expect, test } from "bun:test";

import { treeRowWire, type TreeMark } from "./tree_row";

describe("a row of the building's tree", () => {
  test("a folder says whether it stands open", () => {
    const open: TreeMark = { kind: "directory", open: true };
    const shut: TreeMark = { kind: "directory", open: false };
    expect(treeRowWire(open, () => undefined)["aria-expanded"]).toBe(true);
    expect(treeRowWire(shut, () => undefined)["aria-expanded"]).toBe(false);
  });

  test("a file and a transcript open elsewhere and announce no disclosure", () => {
    const marks: readonly TreeMark[] = [{ kind: "file" }, { kind: "transcript", hint: "opens the run" }];
    expect(marks.map((mark) => treeRowWire(mark, () => undefined)["aria-expanded"])).toEqual([undefined, undefined]);
  });
});
