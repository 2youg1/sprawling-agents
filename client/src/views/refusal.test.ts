// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// A toast's clock: paused while the pointer or the focus holds it, and
// running again only once nothing does (client/Spec.lean §4-35). Before
// this clock the focus did not pause a toast at all, and a pointer
// leaving restarted it under a reader whose focus was still inside.

import { describe, expect, test } from "bun:test";

import type { Clock } from "./refusal";
import { held, released } from "./refusal";

const FRESH: Clock = { left: 8_000, since: 0, holders: [] };

describe("a toast's clock", () => {
  test("the first holder banks the stretch that ran", () => {
    expect(held(FRESH, "pointer", 3_000)).toEqual({ left: 5_000, since: 3_000, holders: ["pointer"] });
  });

  test("a second holder banks nothing, and the clock runs only once both let go", () => {
    const both = held(held(FRESH, "focus", 1_000), "pointer", 4_000);
    expect(both.left).toBe(7_000);
    const pointerGone = released(both, "pointer", 6_000);
    expect(pointerGone.runs).toBe(false);
    const focusGone = released(pointerGone.clock, "focus", 9_000);
    expect(focusGone).toEqual({ clock: { left: 7_000, since: 9_000, holders: [] }, runs: true });
  });

  test("a holder that is not holding changes nothing", () => {
    expect(released(FRESH, "focus", 5_000)).toEqual({ clock: FRESH, runs: false });
    const pointed = held(FRESH, "pointer", 2_000);
    expect(held(pointed, "pointer", 5_000)).toEqual(pointed);
  });

  test("a stretch longer than the life left leaves nothing, never less", () => {
    expect(held(FRESH, "pointer", 20_000).left).toBe(0);
  });
});
