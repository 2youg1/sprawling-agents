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
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { get } from "svelte/store";

import { FACES, FEATURES, FAMILY_SHAPE, featuresOffered } from "./appearance";
import { loadPreferences } from "./prefs";
import { adopted } from "./prefs_city";
import { memory } from "./rows";

const GLASS = "sprawling.appearance.glass";
const BLEND = "sprawling.appearance.blend";

describe("the built-in face", () => {
  test("is Paper Mono, and a cached word no build offers reads back as that default", () => {
    const rows = memory();
    expect(get(loadPreferences(rows, "en").held).appearance.sans).toBe("paper");
    // `geist` is a family this build no longer carries and `maple` is one
    // it does: the first is unknown and reads as the default, the second
    // is the person's own choice and stays (`wire D123`).
    rows.setItem("sprawling.appearance.sans", "geist");
    rows.setItem("sprawling.appearance.mono", "maple");
    const appearance = get(loadPreferences(rows, "en").held).appearance;
    expect([appearance.sans, appearance.mono]).toEqual(["paper", "maple"]);
  });
});

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
    expect([
      read("45"),
      read("90"),
      read("30"),
      read("95"),
      read("20"),
      read("half"),
      read("4.5"),
    ]).toEqual([45, 90, 30, null, null, null, null]);
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
  test("from a city that keeps none of them leaves the reading face, glass and the blend opacity where this browser had them", () => {
    const rows = memory();
    const door = loadPreferences(rows, "en");
    const before = get(door.held);
    const held = {
      ...before,
      appearance: {
        ...before.appearance,
        reading: "libron" as const,
        glass: "off" as const,
        blend: 40,
      },
    };
    const answer = {
      appearance: {
        lighting: "light" as const,
        sans: "system" as const,
        mono: "maple" as const,
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
      mono: "maple",
      sansStack: "",
      monoStack: "",
      reading: "libron",
      fallbacks: [],
      features: [],
      body: null,
      lineHeight: null,
      tracking: null,
      weight: "regular",
      density: "compact",
      chroma: "full",
      motion: "off",
      glass: "off",
      blend: 40,
    });
  });
});

describe("the fallback chain", () => {
  const FIRST = "sprawling.appearance.fallback1";
  const SECOND = "sprawling.appearance.fallback2";

  test("keeps the order it was given, drops nothing between, and has no row for an empty slot", () => {
    const rows = memory();
    const door = loadPreferences(rows, "en");
    const look = get(door.held).appearance;
    door.setAppearance({ ...look, fallbacks: ["Maple Mono NF CN", "Noto Sans SC"] });
    expect(get(loadPreferences(rows, "en").held).appearance.fallbacks).toEqual([
      "Maple Mono NF CN",
      "Noto Sans SC",
    ]);
    // The second name alone closes the gap: a chain is a list, and a list
    // that began at the second slot would read back in the first.
    rows.removeItem(FIRST);
    expect(get(loadPreferences(rows, "en").held).appearance.fallbacks).toEqual(["Noto Sans SC"]);
    rows.removeItem(SECOND);
    expect(rows.getItem(FIRST)).toBeNull();
    expect(get(loadPreferences(rows, "en").held).appearance.fallbacks).toEqual([]);
  });

  test("drops a name that is not one family name rather than splicing a chain out of it", () => {
    const rows = memory();
    rows.setItem(FIRST, 'Maple Mono NF CN", "Inter');
    expect(get(loadPreferences(rows, "en").held).appearance.fallbacks).toEqual([]);
    expect(FAMILY_SHAPE.test("思源宋体")).toBe(true);
    expect(FAMILY_SHAPE.test("Maple Mono NF CN")).toBe(true);
    expect(FAMILY_SHAPE.test("A, B")).toBe(false);
  });
});

describe("the type features", () => {
  const FEATURES_ROW = "sprawling.appearance.features";

  test("read back as the six this build offers, in their declared order, without repeats", () => {
    const rows = memory();
    rows.setItem(FEATURES_ROW, "slashed_zero,duospace,kerning,duospace");
    expect(get(loadPreferences(rows, "en").held).appearance.features).toEqual([
      "duospace",
      "slashed_zero",
    ]);
    rows.setItem(FEATURES_ROW, "");
    expect(get(loadPreferences(rows, "en").held).appearance.features).toEqual([]);
  });

  test("are the faces' own declarations, and every one of them reaches the stylesheet", () => {
    const paper = featuresOffered(["paper"]);
    const maple = featuresOffered(["maple"]);
    expect([paper.size, maple.size]).toEqual([FEATURES.length, 3]);
    expect([featuresOffered(["system"]).size, featuresOffered(["custom"]).size]).toEqual([0, 0]);
    // The words the switches are set under are the wire's spellings, and
    // the stylesheet selects on exactly those words: a feature offered
    // here and unknown there would be a switch that draws nothing.
    const css = readFileSync(
      fileURLToPath(new URL("../theme/preference.css", import.meta.url)),
      "utf8",
    );
    for (const feature of [...paper, ...maple]) {
      expect(css).toContain(`data-features~="${feature}"`);
    }
    // `custom` has no rule on purpose: a stack somebody typed is the one
    // value the stylesheet cannot hold, so it arrives as `--face-sans` on
    // the root element itself.
    for (const face of FACES.filter((each) => each !== "custom")) {
      expect(css).toContain(`data-sans="${face}"`);
      expect(css).toContain(`data-mono="${face}"`);
    }
  });
});

describe("the ratios that set a line", () => {
  const LEADING = "sprawling.appearance.line_height";
  const TRACKING = "sprawling.appearance.tracking";
  const WEIGHT = "sprawling.appearance.weight";

  test("are drawn at the theme's own posture until the person states one, and read back inside their floors", () => {
    const rows = memory();
    expect(get(loadPreferences(rows, "en").held).appearance).toMatchObject({
      lineHeight: null,
      tracking: null,
      weight: "regular",
    });
    rows.setItem(LEADING, "1750");
    rows.setItem(TRACKING, "-10");
    rows.setItem(WEIGHT, "medium");
    expect(get(loadPreferences(rows, "en").held).appearance).toMatchObject({
      lineHeight: 1750,
      tracking: -10,
      weight: "medium",
    });
    // A row under the floor and a row that is not a whole number are both
    // the person saying nothing, which is what the box refuses too.
    rows.setItem(LEADING, "900");
    rows.setItem(TRACKING, "-500");
    rows.setItem(WEIGHT, "bold");
    expect(get(loadPreferences(rows, "en").held).appearance).toMatchObject({
      lineHeight: null,
      tracking: null,
      weight: "regular",
    });
  });
});
