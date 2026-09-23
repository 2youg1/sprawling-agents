// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The five states are told apart by their drawings and only reinforced
// by colour, because a forced-colours mode repaints every fill and
// border and leaves the shapes alone. Two states sharing one paint tier
// is therefore correct and checked past here; two states sharing one
// drawing would leave those machines with nothing to read, and that is
// what the test below holds the line on.

import { describe, expect, test } from "bun:test";

import { statusLook, type Status } from "./glyph";

// The closed set spelled out as its own record, so a sixth state cannot
// be added without touching this file too.
const FIVE: Record<Status, Status> = {
  idle: "idle",
  live: "live",
  waiting: "waiting",
  refused: "refused",
  done: "done",
};

describe("the five states each have their own drawing", () => {
  test("no two states are told apart by colour alone", () => {
    const drawings = Object.values(FIVE).map((state) => statusLook(state).glyph);
    expect(new Set(drawings).size).toBe(drawings.length);
  });
});
