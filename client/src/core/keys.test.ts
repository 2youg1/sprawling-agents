// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { ACTIONS, DEFAULTS, conflictsOf, loadKeys, reserved } from "./keys";
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

  test("Accel-B opens the mailbox and Accel-J the right pane, from a text box too", () => {
    expect(keys.acting(press("b", { ctrlKey: true, target: "field" }))).toBe("mailbox");
    expect(keys.acting(press("j", { ctrlKey: true }))).toBe("inspect");
  });

  // The chord was once dropped with the fork button; branching is its
  // own feature and keeps its key (client/Spec.lean D44).
  test("Accel-Shift-F branches from the entry under the hand, and f alone is typed", () => {
    expect([keys.acting(press("F", { ctrlKey: true, shiftKey: true })), keys.acting(press("f"))]).toEqual(["fork.here", null]);
  });

  // A letter pressed while the focus sat on a message or a card forked
  // the conversation or answered the card for the person; only the key
  // that moves the focus into the message box stays a single key.
  test("every action but focusing the message box holds the accelerator", () => {
    expect(ACTIONS.filter((action) => !DEFAULTS[action].accel)).toEqual(["composer.focus"]);
  });

  test("no action ships on a chord the browser keeps for itself", () => {
    expect(ACTIONS.filter((action) => reserved(DEFAULTS[action]))).toEqual([]);
  });

  test("no two actions ship on one chord", () => {
    expect(conflictsOf(DEFAULTS)).toEqual([]);
  });
});
