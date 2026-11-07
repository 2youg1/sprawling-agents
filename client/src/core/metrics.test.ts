// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// A ratio box refuses exactly what the wire type refuses: a value under
// its floor. There is no ceiling, and an empty box is not a refusal —
// it is the person saying nothing, which is a different act.

import { expect, test } from "bun:test";

import { LINE_HEIGHT_FLOOR, TRACKING_FLOOR, ratioOf, spelledRatio } from "./metrics";
import { LINE_HEIGHT, TRACKING } from "../wire";

test("the boxes read their floors out of the wire, not out of a copy", () => {
  expect([LINE_HEIGHT_FLOOR, TRACKING_FLOOR]).toEqual([LINE_HEIGHT.min, TRACKING.min]);
});

test("a line box is read in thousandths and anything under the floor is refused", () => {
  expect(ratioOf(String(1.2), LINE_HEIGHT_FLOOR)).toEqual({ kind: "set", perMille: 1200 });
  expect(ratioOf("1.625", LINE_HEIGHT_FLOOR)).toEqual({ kind: "set", perMille: 1625 });
  expect(ratioOf("2", LINE_HEIGHT_FLOOR)).toEqual({ kind: "set", perMille: 2000 });
  expect(ratioOf(" 1.25 ", LINE_HEIGHT_FLOOR)).toEqual({ kind: "set", perMille: 1250 });
  expect(ratioOf("1.199", LINE_HEIGHT_FLOOR)).toEqual({ kind: "refused" });
  expect(ratioOf("1.6255", LINE_HEIGHT_FLOOR)).toEqual({ kind: "refused" });
  expect(ratioOf("tall", LINE_HEIGHT_FLOOR)).toEqual({ kind: "refused" });
  expect(ratioOf("", LINE_HEIGHT_FLOOR)).toEqual({ kind: "cleared" });
});

test("a tracking is signed, read in thousandths, and anything under the floor is refused", () => {
  expect(ratioOf("-0.01", TRACKING_FLOOR)).toEqual({ kind: "set", perMille: -10 });
  expect(ratioOf("0", TRACKING_FLOOR)).toEqual({ kind: "set", perMille: 0 });
  expect(ratioOf("0.005", TRACKING_FLOOR)).toEqual({ kind: "set", perMille: 5 });
  expect(ratioOf("-0.051", TRACKING_FLOOR)).toEqual({ kind: "refused" });
  expect(ratioOf("-0.05", TRACKING_FLOOR)).toEqual({ kind: "set", perMille: -50 });
  expect(ratioOf("--1", TRACKING_FLOOR)).toEqual({ kind: "refused" });
  // More than the integer the wire holds is refused here rather than left
  // for the city to refuse the whole appearance over.
  expect(ratioOf("99999999999", LINE_HEIGHT_FLOOR)).toEqual({ kind: "refused" });
});

test("a ratio is spelled back with at most three decimals, and the round trip holds", () => {
  expect([1625, 1200, 2000, 0, -10, 5].map(spelledRatio)).toEqual([
    "1.625",
    "1.2",
    "2",
    "0",
    "-0.01",
    "0.005",
  ]);
  for (const perMille of [1200, 1625, 2000, 0, -10, 5]) {
    expect(ratioOf(spelledRatio(perMille), TRACKING_FLOOR)).toEqual({
      kind: "set",
      perMille,
    });
  }
});
