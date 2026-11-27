// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { lookOf, type ButtonProps } from "./button";

function counted(props: Omit<ButtonProps, "onPress">): {
  readonly props: ButtonProps;
  readonly presses: () => number;
} {
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
      "aria-label": undefined,
      "aria-pressed": undefined,
      onclick: undefined,
    });
    wire.onclick();
    expect(hand.presses()).toBe(1);
  });

  test("a toggle publishes both pressed states and uses the same activation", () => {
    const hand = counted({ label: "Night mode" });
    const off = lookOf({ ...hand.props, pressed: false });
    const on = lookOf({ ...hand.props, pressed: true });
    expect(off.label).toBe(on.label);
    expect(off.wire(undefined)["aria-pressed"]).toBe("false");
    expect(on.wire(undefined)["aria-pressed"]).toBe("true");
    off.wire(undefined).onclick();
    on.wire(undefined).onclick();
    expect(hand.presses()).toBe(2);
  });

  test("a given name is what the control is read by, while the face stays the label", () => {
    const look = lookOf({ label: "forget", name: "forget Firefox · Windows" });
    expect([look.label, look.wire(undefined)["aria-label"]]).toEqual([
      "forget",
      "forget Firefox · Windows",
    ]);
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
    }).toEqual({
      state: "stopped",
      why: "the city is halted",
      disabled: true,
      busy: false,
      described: "hint-7",
    });
    wire.onclick();
    expect(hand.presses()).toBe(0);
  });

  test("tone and type fall back to secondary and button, and a submit stays a submit", () => {
    expect(lookOf({ label: "Go" }).tone).toBe("secondary");
    expect(lookOf({ label: "Go" }).wire(undefined).type).toBe("button");
    expect(lookOf({ label: "Go", type: "submit" }).wire(undefined).type).toBe("submit");
  });
});
