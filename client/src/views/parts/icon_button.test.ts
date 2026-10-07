// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The icon-only key's wiring as client/spec/Views/Parts.lean §7-2
// states it: the accessible name is the label whatever the state, the
// hint shows the name while the key can be used and the reason while it
// cannot, `aria-describedby` points at the hint only when it carries a
// reason (so the name is not read twice), and one guard keeps a press
// from landing on a refused key. The test imports no look.

import { describe, expect, test } from "bun:test";

import { lookOf, type IconButtonProps } from "./icon_button";

function counted(props: Omit<IconButtonProps, "onPress">): { readonly props: IconButtonProps; readonly presses: () => number } {
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

describe("icon button wiring", () => {
  test("a live key is named by its label, hints its label, and lets the press through", () => {
    const hand = counted({ glyph: "reveal", label: "show in the file manager" });
    const look = lookOf(hand.props);
    const wire = look.wire("tip-1");
    expect({ hint: look.hint, glyph: look.glyph, wire: { ...wire, onclick: undefined } }).toEqual({
      hint: "show in the file manager",
      glyph: "reveal",
      wire: {
        type: "button",
        "aria-label": "show in the file manager",
        "aria-disabled": false,
        "aria-describedby": undefined,
        onclick: undefined,
      },
    });
    wire.onclick();
    expect(hand.presses()).toBe(1);
  });

  test("a refused key keeps its name, hints the reason, points at it, and swallows the press", () => {
    const hand = counted({ glyph: "reveal", label: "show in the file manager", why: "this build cannot open it" });
    const look = lookOf(hand.props);
    const wire = look.wire("tip-2");
    expect({
      hint: look.hint,
      name: wire["aria-label"],
      disabled: wire["aria-disabled"],
      described: wire["aria-describedby"],
    }).toEqual({
      hint: "this build cannot open it",
      name: "show in the file manager",
      disabled: true,
      described: "tip-2",
    });
    wire.onclick();
    expect(hand.presses()).toBe(0);
  });
});
