// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What a progress bar tells a screen reader, against the progress row
// of `client/spec/Views/Parts.lean` §7-1: the upper bound and the
// current value appear exactly when there is an end, and the bar is
// busy exactly when there is not.

import { describe, expect, test } from "bun:test";

import { lookOf } from "./progress";

describe("a progress bar", () => {
  test("with an end states both values and is not busy", () => {
    expect(lookOf({ label: "files walked", done: 3, total: 12 })).toEqual({
      bar: {
        role: "progressbar",
        "aria-label": "files walked",
        "aria-valuemin": 0,
        "aria-valuemax": 12,
        "aria-valuenow": 3,
        "aria-busy": "false",
      },
      reached: { done: 3, total: 12, share: 0.25 },
    });
  });

  test("without an end states neither value and is busy", () => {
    for (const total of [0, -1]) {
      expect(lookOf({ label: "files walked", done: 3, total })).toEqual({
        bar: { role: "progressbar", "aria-label": "files walked", "aria-valuemin": 0, "aria-busy": "true" },
        reached: undefined,
      });
    }
  });

  test("fills no less than nothing and no more than the track", () => {
    expect([
      lookOf({ label: "", done: -4, total: 10 }).reached?.share,
      lookOf({ label: "", done: 14, total: 10 }).reached?.share,
    ]).toEqual([0, 1]);
  });
});
