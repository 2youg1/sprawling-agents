// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The desktop allowlist editor's wiring, read without its look: typing
// hands the box's text to the seat, the save says why it cannot be used
// until something changed, and the two sentences about the file appear
// only for the answer they describe.

import { describe, expect, test } from "bun:test";

import { Address } from "../wire";
import { lookOf } from "./desktop";
import type { DesktopState, Hands } from "./desktop";

const HALL = Address.make("hall");

function hands(): { readonly typed: string[]; readonly saves: number[]; readonly hands: Hands } {
  const typed: string[] = [];
  const saves: number[] = [];
  return { typed, saves, hands: { type: (text) => typed.push(text), save: () => saves.push(1) } };
}

const at = (state: Partial<DesktopState>): DesktopState => ({ addr: HALL, draft: "", edited: false, file: "held", ...state });

describe("the allowlist editor", () => {
  test("typing hands the text to the seat, and the box shows the draft", () => {
    const seat = hands();
    const look = lookOf(at({ draft: "notepad" }), "en", seat.hands);
    look.box.oninput({ currentTarget: { value: "notepad\ncalc" } });
    expect([look.box.value, seat.typed]).toEqual(["notepad", ["notepad\ncalc"]]);
  });

  test("the save says why until something changed, and then saves", () => {
    const seat = hands();
    expect(lookOf(at({}), "en", seat.hands).save.why).toBe("there is nothing to save yet");
    const edited = lookOf(at({ edited: true }), "en", seat.hands).save;
    edited.press();
    expect([edited.why, seat.saves]).toEqual([undefined, [1]]);
  });

  test("a missing file names where a save creates it; an empty one says so until somebody types", () => {
    const seat = hands();
    expect(lookOf(at({ file: "missing" }), "en", seat.hands).missing).toContain("hall/.sprawling/DESKTOP.toml");
    expect(lookOf(at({ file: "held" }), "en", seat.hands).missing).toBeUndefined();
    expect(lookOf(at({ file: "empty" }), "en", seat.hands).none).toBeDefined();
    expect(lookOf(at({ file: "empty", edited: true }), "en", seat.hands).none).toBeUndefined();
  });
});
