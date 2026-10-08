// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { ACTIONS, DEFAULTS, browserKeeps, conflictsOf, loadKeys, reserved } from "./keys";
import type { Pressed } from "./press";
import { loadPreferences } from "./prefs";
import { memory } from "./rows";

function press(key: string, held: Partial<Pressed> = {}): Pressed {
  return { key, ctrlKey: false, metaKey: false, shiftKey: false, altKey: false, target: "page", ...held };
}

describe("the shell's own keys", () => {
  const keys = loadKeys(loadPreferences(memory(), "en"), "Windows");

  test("Accel-backslash changes the tier, and the backslash alone is typed", () => {
    expect(keys.acting(press("\\", { ctrlKey: true }))).toBe("tier.cycle");
    expect([keys.acting(press("\\")), keys.acting(press("\\", { target: "field" }))]).toEqual([null, null]);
  });

  test("Accel-slash opens the palette, from a text box too", () => {
    expect(keys.acting(press("/", { ctrlKey: true, target: "field" }))).toBe("palette");
  });

  // A letter pressed while the focus sat on a message or a card forked
  // the conversation or answered the card for the person; only the key
  // that moves the focus into the message box stays a single key.
  test("every bound action but focusing the message box holds the accelerator", () => {
    expect(ACTIONS.filter((action) => DEFAULTS[action]?.accel === false)).toEqual(["composer.focus"]);
  });

  test("no action ships on a chord the browser keeps for itself", () => {
    expect(ACTIONS.filter((action) => reserved(DEFAULTS[action]))).toEqual([]);
  });

  // The audit of the four browsers' shortcut lists (client/Spec.lean D-keys):
  // a default that a browser gives a function of its own is moved or dropped.
  test("no action ships on a chord a browser gives a function of its own", () => {
    expect(ACTIONS.filter((action) => browserKeeps(DEFAULTS[action]))).toEqual([]);
  });

  test("a chord the person binds reaches its action even where a browser uses it", () => {
    const kept = loadKeys(loadPreferences(memory(), "en"), "Windows");
    kept.bind("finder", { accel: true, shift: false, key: "p" });
    expect(kept.acting(press("p", { ctrlKey: true }))).toBe("finder");
  });

  test("no two actions ship on one chord", () => {
    expect(conflictsOf(DEFAULTS)).toEqual([]);
  });
});
