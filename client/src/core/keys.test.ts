// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { DEFAULTS, conflictsOf, loadKeys } from "./keys";
import type { Pressed } from "./press";
import { loadPreferences } from "./prefs";
import { memory } from "./rows";

function press(key: string, held: Partial<Pressed> = {}): Pressed {
  return { key, ctrlKey: false, metaKey: false, shiftKey: false, altKey: false, target: "page", ...held };
}

describe("the shell's own keys", () => {
  const keys = loadKeys(loadPreferences(memory(), "en"), "Windows");

  test("the backslash changes the tier on the page and is typed inside a text box", () => {
    expect(keys.acting(press("\\"))).toBe("tier.cycle");
    expect(keys.acting(press("\\", { target: "field" }))).toBeNull();
  });

  test("Accel-B opens the mailbox and Accel-J the right pane, from a text box too", () => {
    expect(keys.acting(press("b", { ctrlKey: true, target: "field" }))).toBe("mailbox");
    expect(keys.acting(press("j", { ctrlKey: true }))).toBe("inspect");
  });

  // The chord was once dropped with the fork button; branching is its
  // own feature and keeps its key (client/Spec.lean D44).
  test("f branches from the entry under the hand on the page and is typed inside a text box", () => {
    expect([keys.acting(press("f")), keys.acting(press("f", { target: "field" }))]).toEqual(["fork.here", null]);
  });

  test("no two actions ship on one chord", () => {
    expect(conflictsOf(DEFAULTS)).toEqual([]);
  });
});
