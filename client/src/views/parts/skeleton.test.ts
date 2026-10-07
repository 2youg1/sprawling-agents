// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What a skeleton tells a screen reader and how many bars it stands up,
// against the skeleton row of `client/spec/Views/Parts.lean` §7-1.

import { describe, expect, test } from "bun:test";

import { lookOf } from "./skeleton";

describe("a skeleton", () => {
  test("announces itself once and hides every bar", () => {
    const look = lookOf({ label: "checking the machine", rows: 2 });
    expect([look.block, look.bars.map((bar) => bar.wire)]).toEqual([
      { role: "status", "aria-label": "checking the machine", "aria-busy": "true" },
      [{ "aria-hidden": "true" }, { "aria-hidden": "true" }],
    ]);
  });

  test("stands up at least one bar, cycling the four widths of prose", () => {
    expect([
      lookOf({ label: "", rows: 0 }).bars.map((bar) => bar.width),
      lookOf({ label: "", rows: 5 }).bars.map((bar) => bar.width),
    ]).toEqual([["whole"], ["whole", "five-sixths", "two-thirds", "three-quarters", "whole"]]);
  });
});
