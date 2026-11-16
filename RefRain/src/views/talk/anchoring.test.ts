// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { FOOT_TOLERANCE, anchorAt } from "./anchoring";
import type { Foot, Moment } from "./anchoring";

function foot(distance: number): Moment {
  const measured: Foot = { scrollTop: 0, clientHeight: 240, scrollHeight: 240 + distance };
  return { kind: "scrolled", foot: measured };
}

describe("where the conversation view is anchored", () => {
  test("opening a room follows to the foot whatever the scroller measured", () => {
    expect(anchorAt({ kind: "opened" })).toBe("follow");
  });

  test("the foot itself follows the growing edge", () => {
    expect(anchorAt(foot(0))).toBe("follow");
  });

  test("closer to the foot than the tolerance still follows", () => {
    expect(anchorAt(foot(FOOT_TOLERANCE - 1))).toBe("follow");
  });

  test("the tolerance itself is a place the person chose", () => {
    expect(anchorAt(foot(FOOT_TOLERANCE))).toBe("hold");
  });

  test("reading back holds the place", () => {
    expect(anchorAt(foot(600))).toBe("hold");
  });

  test("a stretch past the foot follows", () => {
    expect(anchorAt(foot(-24))).toBe("follow");
  });
});
