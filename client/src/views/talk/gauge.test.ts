// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The wiring the context ring and a sessions row's bar hand their looks:
// the meter's values (client/spec/Views/Workspace.lean, the ring's
// `aria-*`), which checkpoints each look is given, and that the bar
// marks only the handoff.

import { describe, expect, test } from "bun:test";
import { fill, say } from "../../core/lang";
import { barOf, ringOf } from "./gauge";
import type { Context } from "./gauge";

const BOTH: Context = { used: 82_400, window: 200_000, first: 30, second: 65 };

describe("the context ring's look", () => {
  test("is the meter of used against the window, with both checkpoints", () => {
    const reading = [
      fill(say("en", "ring_used"), { used: "82.4k", window: "200k" }),
      fill(say("en", "ring_share"), { n: "41" }),
      fill(say("en", "ring_first"), { n: "30" }),
      fill(say("en", "ring_second"), { n: "65" }),
    ].join(" · ");
    expect(ringOf("en", BOTH)).toEqual({
      meter: {
        role: "meter",
        tabindex: 0,
        "aria-label": say("en", "ring_name"),
        "aria-valuemin": 0,
        "aria-valuemax": 200_000,
        "aria-valuenow": 82_400,
        "aria-valuetext": reading,
      },
      reading,
      arc: { dasharray: "59 100", dashoffset: "-41" },
      checkpoints: [
        { reminder: "first", at: 30 },
        { reminder: "second", at: 65 },
      ],
    });
  });

  test("holds the meter's value inside its range when a turn overran the window", () => {
    const ring = ringOf("en", { ...BOTH, used: 250_000 });
    expect(ring?.meter["aria-valuenow"]).toBe(200_000);
    expect(ring?.arc).toEqual({ dasharray: "0 100", dashoffset: "-100" });
  });

  test("puts no checkpoint for a reminder the city did not state", () => {
    expect(ringOf("en", { ...BOTH, first: null })?.checkpoints).toEqual([{ reminder: "second", at: 65 }]);
  });

  test("is absent where the model stated no window", () => {
    expect(ringOf("en", null)).toBeNull();
  });
});

describe("a sessions row's context bar", () => {
  test("marks only the handoff, at its percent", () => {
    expect(barOf("en", BOTH)).toEqual({
      share: 41,
      handoff: { reminder: "second", at: 65 },
      said: fill(say("en", "ring_share"), { n: "41" }),
    });
  });

  test("marks nothing where the room states no handoff", () => {
    expect(barOf("en", { ...BOTH, second: null })?.handoff).toBeUndefined();
  });
});
