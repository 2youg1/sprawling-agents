// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { FOOT_TOLERANCE, anchorAt } from "./anchoring";
import type { Foot, Moment } from "./anchoring";

// A viewport of 240px whose distance from the foot is `distance`: the
// three measurements travel together, so the fixture names the one
// reading that varies and fixes the rest.
function foot(distance: number): Moment {
  const measured: Foot = { scrollTop: 0, clientHeight: 240, scrollHeight: 240 + distance };
  return { kind: "scrolled", foot: measured };
}

describe("where the conversation view is anchored", () => {
  // A room opens with its scroller at the top, far from the foot, and
  // that position is the browser's, not a place the person chose to read.
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

  // Rubber-band scrolling carries the viewport past the foot before it
  // springs back; that is still the foot, not a place to hold.
  test("a stretch past the foot follows", () => {
    expect(anchorAt(foot(-24))).toBe("follow");
  });
});
