// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The button's wiring as client/spec/Views/Parts.lean §7-2 states it:
// `aria-disabled` while loading or refused, `aria-busy` only while
// loading, `aria-describedby` pointing at the hint that carries the
// reason, and one guard that keeps a press from landing unless the
// control is idle. These hold for any look that spreads the bag, which
// is why the test imports no look.

import { describe, expect, test } from "bun:test";

import { lookOf, type ButtonProps } from "./button";

function counted(props: Omit<ButtonProps, "onPress">): { readonly props: ButtonProps; readonly presses: () => number } {
  let presses = 0;
  return {
    props: {
      ...props,
      onPress: () => {
        presses += 1;
      },
    },
    presses: () => presses,
  };
}

describe("button wiring", () => {
  test("an idle button is enabled, not busy, points at nothing and lets the press through", () => {
    const hand = counted({ label: "Save", tone: "primary" });
    const look = lookOf(hand.props);
    const wire = look.wire(undefined);
    expect({ state: look.state, tone: look.tone, why: look.why }).toEqual({
      state: "idle",
      tone: "primary",
      why: undefined,
    });
    expect({ ...wire, onclick: undefined }).toEqual({
      type: "button",
      "aria-disabled": false,
      "aria-busy": false,
      "aria-describedby": undefined,
      onclick: undefined,
    });
    wire.onclick();
    expect(hand.presses()).toBe(1);
  });

  test("a loading button is disabled and busy, and swallows the press", () => {
    const hand = counted({ label: "Save", loading: true });
    const look = lookOf(hand.props);
    const wire = look.wire(undefined);
    expect([look.state, wire["aria-disabled"], wire["aria-busy"]]).toEqual(["loading", true, true]);
    wire.onclick();
    expect(hand.presses()).toBe(0);
  });

  test("a refused button names its hint, is disabled but not busy, and swallows the press", () => {
    const hand = counted({ label: "Save", loading: true, why: "the city is halted" });
    const look = lookOf(hand.props);
    const wire = look.wire("hint-7");
    expect({
      state: look.state,
      why: look.why,
      disabled: wire["aria-disabled"],
      busy: wire["aria-busy"],
      described: wire["aria-describedby"],
    }).toEqual({ state: "stopped", why: "the city is halted", disabled: true, busy: false, described: "hint-7" });
    wire.onclick();
    expect(hand.presses()).toBe(0);
  });

  test("tone and type fall back to secondary and button, and a submit stays a submit", () => {
    expect(lookOf({ label: "Go" }).tone).toBe("secondary");
    expect(lookOf({ label: "Go" }).wire(undefined).type).toBe("button");
    expect(lookOf({ label: "Go", type: "submit" }).wire(undefined).type).toBe("submit");
  });
});
