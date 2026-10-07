// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The wiring of the control that records a shortcut, through the bag
// any look spreads on its button: what a press does while it listens,
// and what it leaves alone while it does not.

import { describe, expect, test } from "bun:test";

import type { Action, Chord } from "../../core/keys";
import { chordOf } from "./chord";
import type { ChordHands, ChordPress } from "./chord";

const ACTION: Action = "palette";

interface Heard {
  readonly listen: (Action | null)[];
  readonly bound: [Action, Chord][];
  prevented: number;
}

function record(): { readonly hands: ChordHands; readonly heard: Heard } {
  const heard: Heard = { listen: [], bound: [], prevented: 0 };
  return {
    heard,
    hands: {
      listen: (action) => heard.listen.push(action),
      bind: (action, chord) => heard.bound.push([action, chord]),
    },
  };
}

function press(heard: Heard, key: string, held: Partial<Omit<ChordPress, "key" | "preventDefault">> = {}): ChordPress {
  return {
    key,
    ctrlKey: held.ctrlKey ?? false,
    metaKey: held.metaKey ?? false,
    shiftKey: held.shiftKey ?? false,
    preventDefault: () => {
      heard.prevented += 1;
    },
  };
}

const PROPS = { action: ACTION, spelled: "accel+k", prompt: "press a chord", tip: "change" } as const;

describe("chordOf", () => {
  test("a click starts listening and a blur stops it", () => {
    const { hands, heard } = record();
    const look = chordOf({ ...PROPS, listening: false }, hands);
    look.wire.onclick();
    look.wire.onblur();
    expect(heard).toEqual({ listen: [ACTION, null], bound: [], prevented: 0 });
  });

  test("while it does not listen, every key is the page's", () => {
    const { hands, heard } = record();
    chordOf({ ...PROPS, listening: false }, hands).wire.onkeydown(press(heard, "k", { ctrlKey: true }));
    expect(heard).toEqual({ listen: [], bound: [], prevented: 0 });
  });

  test("a modifier alone keeps it listening and is not taken from the page", () => {
    const { hands, heard } = record();
    chordOf({ ...PROPS, listening: true }, hands).wire.onkeydown(press(heard, "Shift", { shiftKey: true }));
    expect(heard).toEqual({ listen: [], bound: [], prevented: 0 });
  });

  test("Escape stops listening and binds nothing", () => {
    const { hands, heard } = record();
    chordOf({ ...PROPS, listening: true }, hands).wire.onkeydown(press(heard, "Escape"));
    expect(heard).toEqual({ listen: [null], bound: [], prevented: 1 });
  });

  test("a chord with Cmd or Ctrl binds the accelerator and keeps Shift", () => {
    const { hands, heard } = record();
    const look = chordOf({ ...PROPS, listening: true }, hands);
    look.wire.onkeydown(press(heard, "K", { metaKey: true, shiftKey: true }));
    expect(heard).toEqual({
      listen: [null],
      bound: [[ACTION, { accel: true, shift: true, key: "K" }]],
      prevented: 1,
    });
  });

  test("a chord without the accelerator drops Shift, which the character already says", () => {
    const { hands, heard } = record();
    chordOf({ ...PROPS, listening: true }, hands).wire.onkeydown(press(heard, "?", { shiftKey: true }));
    expect(heard.bound).toEqual([[ACTION, { accel: false, shift: false, key: "?" }]]);
  });

  test("the bag says whether it listens, and the look is told what to draw", () => {
    const { hands } = record();
    const listening = chordOf({ ...PROPS, listening: true }, hands);
    const idle = chordOf({ ...PROPS, listening: false }, hands);
    expect([listening.wire["aria-pressed"], listening.listening]).toEqual([true, "press a chord"]);
    expect([idle.wire["aria-pressed"], idle.listening]).toEqual([false, undefined]);
  });
});
