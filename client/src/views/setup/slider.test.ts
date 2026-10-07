// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The bag any slider look spreads on its range input: an unstated
// figure is read as no figure at all rather than as the range's start,
// which is only where the thumb rests.

import { describe, expect, test } from "bun:test";

import { sliderOf } from "./slider";

const RANGE = { min: 0, max: 60, step: 5 };
const figure = (at: number): string => `${String(at)}%`;

describe("sliderOf", () => {
  test("a stated figure is drawn and read out", () => {
    const moved: string[] = [];
    const look = sliderOf({ label: "blend", range: RANGE, at: 40, figure, onMove: (raw) => moved.push(raw) });
    look.wire.oninput({ currentTarget: { value: "45" } });
    expect({ ...look, wire: { ...look.wire, oninput: undefined } }).toEqual({
      shown: "40%",
      wire: {
        type: "range",
        min: 0,
        max: 60,
        step: 5,
        value: 40,
        "aria-label": "blend",
        "aria-valuetext": "40%",
        oninput: undefined,
      },
    });
    expect(moved).toEqual(["45"]);
  });

  test("with no figure the thumb rests at the start and nothing is said", () => {
    const look = sliderOf({ label: "blend", range: RANGE, at: null, figure, onMove: () => undefined });
    expect([look.shown, look.wire.value, look.wire["aria-valuetext"]]).toEqual(["", 0, undefined]);
  });
});
