// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { loadPreferences } from "./prefs";
import type { Preferences } from "./prefs";
import { memory } from "./rows";

// A record that differs from the shipped postures in every field, so a
// value that failed to travel is visible rather than accidentally
// equal to the fallback.
const STATED: Preferences = {
  lang: "zh",
  welcomed: true,
  panel: false,
  appearance: {
    lighting: "light",
    sans: "system",
    mono: "custom",
    sansStack: "Iosevka",
    monoStack: "Iosevka Term",
    body: 17,
    density: "compact",
    chroma: "off",
    motion: "on",
  },
  proxying: "always",
};

describe("the cache in front of the city", () => {
  test("a record written by one door is read back whole by the next", () => {
    const rows = memory();
    loadPreferences(rows, "en").adopt(STATED);
    // A second door over the same store is the next first paint: it
    // reads the cache and nothing else.
    expect(loadPreferences(rows, "en").held()).toEqual(STATED);
  });

  test("a size nobody stated leaves no row behind for the next paint to find", () => {
    const rows = memory();
    const door = loadPreferences(rows, "en");
    door.setAppearance({ ...STATED.appearance, body: 17 });
    door.setAppearance({ ...STATED.appearance, body: null });
    expect(loadPreferences(rows, "en").held().appearance.body).toBeNull();
  });

  test("a word this build no longer offers is dropped rather than repaired", () => {
    const rows = memory();
    loadPreferences(rows, "en").adopt(STATED);
    rows.setItem("sprawling.appearance.lighting", "sepia");
    expect(loadPreferences(rows, "en").held().appearance.lighting).toBe("system");
  });

  test("a browser asking in Chinese is answered in Chinese before anything is stored", () => {
    expect(loadPreferences(memory(), "zh-Hans-CN").held().lang).toBe("zh");
    expect(loadPreferences(memory(), "en-GB").held().lang).toBe("en");
  });
});

describe("who is keeping these", () => {
  test("a browser the city has not answered keeps them itself, and says so", () => {
    const door = loadPreferences(memory(), "en");
    expect(door.keeper()).toBe("browser");
    door.setLang("zh");
    // A change made before the city answers does not promote this
    // browser to the authority: it is still the only keeper.
    expect(door.keeper()).toBe("browser");
    expect(door.held().lang).toBe("zh");
  });

  test("the city's answer replaces what this browser held, whole", () => {
    const door = loadPreferences(memory(), "en");
    door.setLang("en");
    door.setPanel(true);
    door.adopt(STATED);
    expect(door.held()).toEqual(STATED);
    expect(door.keeper()).toBe("city");
  });

  test("a change made after the city answered leaves the city the keeper", () => {
    const door = loadPreferences(memory(), "en");
    door.adopt(STATED);
    door.setProxying("never");
    expect(door.keeper()).toBe("city");
    expect(door.held().proxying).toBe("never");
  });
});

describe("the two families read by name", () => {
  test("a chord returned to the one this build ships leaves no row", () => {
    const rows = memory();
    const door = loadPreferences(rows, "en");
    door.setChord("go.city", "accel+2");
    expect(door.chord("go.city")).toBe("accel+2");
    door.setChord("go.city", "");
    expect(door.chord("go.city")).toBe("");
    expect(loadPreferences(rows, "en").chord("go.city")).toBe("");
  });

  test("a draft is kept per place a person writes", () => {
    const door = loadPreferences(memory(), "en");
    door.setDraft("hall/mayor", "half a sentence");
    door.setDraft("hall/clerk", "another");
    expect(door.draft("hall/mayor")).toBe("half a sentence");
    expect(door.draft("hall/clerk")).toBe("another");
    expect(door.draft("hall/nobody")).toBe("");
  });
});
