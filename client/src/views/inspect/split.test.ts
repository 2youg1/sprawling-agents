// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The line between the editor and the terminal: the APG Window Splitter
// row of client/Spec.lean §7-11, read off the value the seat hands any
// look (client D95). No look is imported here.

import { describe, expect, test } from "bun:test";

import { lookOf, type Grip, type SplitLook, type SplitProps } from "./split";

interface Heard {
  readonly lines: number[];
  resets: number;
  readonly captured: number[];
}

function drawn(lines: number, grip: { from: Grip | null } = { from: null }): { readonly look: SplitLook; readonly heard: Heard } {
  const heard: Heard = { lines: [], resets: 0, captured: [] };
  const props: SplitProps = {
    lines,
    least: 6,
    most: 20,
    controls: "inspector-editor",
    step: 24,
    onLines: (next) => heard.lines.push(next),
    onReset: () => {
      heard.resets += 1;
    },
  };
  const look = lookOf(props, "move the line", {
    capture: (pointer) => heard.captured.push(pointer),
    hold: () => undefined,
    grip,
  });
  return { look, heard };
}

function press(look: SplitLook, key: string): boolean {
  let prevented = false;
  look.wire.onkeydown({
    key,
    preventDefault: () => {
      prevented = true;
    },
  });
  return prevented;
}

describe("the keys move the line a whole line at a time, inside its bounds", () => {
  test("arrows step, Home and End go to the bounds, Enter gives the proportions back", () => {
    const { look, heard } = drawn(10);
    for (const key of ["ArrowUp", "ArrowDown", "Home", "End", "Enter"]) expect(press(look, key)).toBe(true);
    expect(heard.lines).toEqual([9, 11, 6, 20]);
    expect(heard.resets).toBe(1);
  });

  test("an arrow at a bound stays there", () => {
    const least = drawn(6);
    press(least.look, "ArrowUp");
    const most = drawn(20);
    press(most.look, "ArrowDown");
    expect([...least.heard.lines, ...most.heard.lines]).toEqual([6, 20]);
  });

  test("a key the line does not take is left to the page", () => {
    const { look, heard } = drawn(10);
    expect(press(look, "ArrowLeft")).toBe(false);
    expect(heard.lines).toEqual([]);
  });
});

describe("a drag counts whole lines of the pointer's travel from where it began", () => {
  test("the line follows the pointer and lets go on release", () => {
    const grip: { from: Grip | null } = { from: null };
    const start = drawn(10, grip);
    start.look.wire.onpointerdown({ clientY: 100, pointerId: 7 });
    expect(start.heard.captured).toEqual([7]);
    // The seat draws the look again as the lines move; the drag keeps
    // its beginning across the redraw.
    const moved = drawn(11, grip);
    moved.look.wire.onpointermove({ clientY: 100 + 3 * 24 + 5, pointerId: 7 });
    moved.look.wire.onpointermove({ clientY: 100 - 30 * 24, pointerId: 7 });
    moved.look.wire.onpointerup();
    moved.look.wire.onpointermove({ clientY: 0, pointerId: 7 });
    expect(moved.heard.lines).toEqual([13, 6]);
  });
});

describe("the bag carries the splitter's values", () => {
  test("role, orientation, bounds and the region it sizes", () => {
    const { wire } = drawn(10).look;
    expect([wire.role, wire.tabindex, wire["aria-orientation"], wire["aria-controls"]]).toEqual([
      "separator",
      0,
      "horizontal",
      "inspector-editor",
    ]);
    expect([wire["aria-valuemin"], wire["aria-valuenow"], wire["aria-valuemax"], wire["aria-label"]]).toEqual([
      6,
      10,
      20,
      "move the line",
    ]);
  });
});
