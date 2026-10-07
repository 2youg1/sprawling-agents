// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The two geometry rules of a layer, checked against the properties
// `client/spec/Views/Parts/Popover.lean` proves for them, over every
// input in a small grid rather than over a few picked cases: the proofs
// quantify over all inputs, and an implementation that drifted on one
// corner would pass any short list of examples that missed it.

import { describe, expect, test } from "bun:test";

import { opensOn, reveal } from "./layer";

const SPAN = 9;
const range = (to: number): readonly number[] => Array.from({ length: to }, (_, at) => at);

describe("reveal", () => {
  test("a row that fits the viewport is wholly visible after the scroll (a_fitting_row_is_visible)", () => {
    const missed: string[] = [];
    for (const top of range(SPAN))
      for (const bottom of range(SPAN))
        for (const scroll of range(SPAN))
          for (const height of range(SPAN)) {
            if (bottom < top || bottom > top + height) continue;
            const at = reveal(top, bottom, scroll, height);
            if (!(at <= top && bottom <= at + height)) missed.push(`${String([top, bottom, scroll, height])} -> ${String(at)}`);
          }
    expect(missed).toEqual([]);
  });

  test("a non-empty viewport always reaches a non-empty row (a_nonempty_viewport_reaches_the_row)", () => {
    const missed: string[] = [];
    for (const top of range(SPAN))
      for (const bottom of range(SPAN))
        for (const scroll of range(SPAN))
          for (const height of range(SPAN)) {
            if (!(top < bottom) || height === 0) continue;
            const at = reveal(top, bottom, scroll, height);
            if (!(at < bottom && top < at + height)) missed.push(`${String([top, bottom, scroll, height])} -> ${String(at)}`);
          }
    expect(missed).toEqual([]);
  });

  test("a row already in view leaves the scroll where it was", () => {
    expect(reveal(3, 5, 2, 4)).toBe(2);
  });
});

describe("opensOn", () => {
  test("a layer that fits on its own side stays there (a_fitting_side_is_kept)", () => {
    const moved: string[] = [];
    for (const seen of [true, false])
      for (const here of range(SPAN))
        for (const there of range(SPAN))
          for (const height of range(SPAN)) {
            if (height > here) continue;
            if (opensOn(seen, here, there, height) !== "preferred") moved.push(String([seen, here, there, height]));
          }
    expect(moved).toEqual([]);
  });

  test("a layer moves only to a side with more room, when its own side is too short (a_flip_gains_room)", () => {
    const wrong: string[] = [];
    for (const seen of [true, false])
      for (const here of range(SPAN))
        for (const there of range(SPAN))
          for (const height of range(SPAN)) {
            if (opensOn(seen, here, there, height) !== "other") continue;
            if (!(here < height && here < there)) wrong.push(String([seen, here, there, height]));
          }
    expect(wrong).toEqual([]);
  });

  test("an anchor outside its clipping box keeps the preferred side", () => {
    expect(opensOn(false, 0, 500, 300)).toBe("preferred");
    expect(opensOn(true, 0, 500, 300)).toBe("other");
  });
});
