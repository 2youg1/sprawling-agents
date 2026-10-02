// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The two appearance choices the city's record has no field for: glass,
// and the world layer's opacity in the blend tier (docs/frontend-method.md §4-43).
// Both live in this browser alone, so what has to hold is that a stored
// row reads back inside its domain and that the city's answer, which
// says nothing about either, leaves them where the browser had them.

import { describe, expect, test } from "bun:test";
import { get } from "svelte/store";

import { loadPreferences } from "./prefs";
import { adopted } from "./prefs_city";
import { memory } from "./rows";

const GLASS = "sprawling.appearance.glass";
const BLEND = "sprawling.appearance.blend";

describe("glass", () => {
  test("is drawn until the person turns it off, and an unknown word reads as drawn", () => {
    const rows = memory();
    expect(get(loadPreferences(rows, "en").held).appearance.glass).toBe("on");
    rows.setItem(GLASS, "off");
    expect(get(loadPreferences(rows, "en").held).appearance.glass).toBe("off");
    rows.setItem(GLASS, "frosted");
    expect(get(loadPreferences(rows, "en").held).appearance.glass).toBe("on");
  });
});

describe("the blend tier's opacity", () => {
  test("a stored percent inside the slider's domain is read back, and anything else as unstated", () => {
    const rows = memory();
    const read = (raw: string): number | null => {
      rows.setItem(BLEND, raw);
      return get(loadPreferences(rows, "en").held).appearance.blend;
    };
    expect([read("45"), read("90"), read("30"), read("95"), read("20"), read("half"), read("4.5")]).toEqual([
      45,
      90,
      30,
      null,
      null,
      null,
      null,
    ]);
  });

  test("an opacity nobody stated leaves no row behind for the next paint to find", () => {
    const rows = memory();
    const door = loadPreferences(rows, "en");
    const look = get(door.held).appearance;
    door.setAppearance({ ...look, blend: 40 });
    expect(get(loadPreferences(rows, "en").held).appearance.blend).toBe(40);
    door.setAppearance({ ...look, blend: null });
    expect(rows.getItem(BLEND)).toBeNull();
  });
});

describe("the city's answer", () => {
  test("from a city that keeps neither leaves glass and the blend opacity where this browser had them", () => {
    const rows = memory();
    const door = loadPreferences(rows, "en");
    const before = get(door.held);
    const held = { ...before, appearance: { ...before.appearance, glass: "off" as const, blend: 40 } };
    const answer = {
      appearance: {
        lighting: "light" as const,
        sans: "system" as const,
        mono: "geist" as const,
        sans_stack: "",
        mono_stack: "",
        body_px: null,
        density: "compact" as const,
        chroma: "full" as const,
        motion: "off" as const,
      },
    };
    expect(adopted(held, answer).appearance).toEqual({
      lighting: "light",
      sans: "system",
      mono: "geist",
      sansStack: "",
      monoStack: "",
      body: null,
      density: "compact",
      chroma: "full",
      motion: "off",
      glass: "off",
      blend: 40,
    });
  });
});
