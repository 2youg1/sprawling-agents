// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";
import { get } from "svelte/store";

import { loadPreferences } from "./prefs";
import { adopted, appearanceOnWire } from "./prefs_city";
import { memory } from "./rows";
import type { PreferencePatch } from "../wire";

describe("the preferences the city keeps", () => {
  test("the city's answer is taken over what this browser held, except the tier this tab chose", () => {
    const held = get(loadPreferences(memory(), "en").held);
    const answer = {
      lang: "zh",
      panel: false,
      tier: "panorama",
      proxying: "always",
      appearance: {
        lighting: "light",
        sans: "system",
        mono: "custom",
        sans_stack: "Iosevka",
        mono_stack: "Iosevka Term",
        body_px: 17,
        density: "compact",
        chroma: "off",
        motion: "on",
        glass: "off",
        blend_percent: 45,
      },
    } as const;

    expect(adopted(held, answer)).toEqual({
      ...held,
      lang: "zh",
      panel: false,
      proxying: "always",
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
        glass: "off",
        blend: 45,
      },
    });
  });

  test("the city's colours replace this browser's, and a city that states none leaves them", () => {
    const door = loadPreferences(memory(), "en");
    door.setTheme({ tokens: { "--color-g2": "#202020" }, css: null });
    const held = get(door.held);

    expect(adopted(held, { theme: { tokens: { "--color-accent": "#3366ff" } } }).theme).toEqual({
      tokens: { "--color-accent": "#3366ff" },
      css: null,
    });
    expect(adopted(held, {}).theme).toEqual(held.theme);
  });

  test("a city that never heard of the tier or the glass leaves this browser's, and an opacity outside the slider is no opacity", () => {
    const door = loadPreferences(memory(), "en");
    door.setTier("zen");
    door.setAppearance({ ...get(door.held).appearance, glass: "off", blend: 60 });
    const held = get(door.held);
    const answer = { tier: null, appearance: { ...appearanceOnWire(held.appearance), glass: null, blend_percent: 7 } };

    expect(adopted(held, answer)).toEqual({ ...held, appearance: { ...held.appearance, blend: null } });
  });

  test("each named change but the tier is told to the city as its own patch", () => {
    const door = loadPreferences(memory(), "en");
    const told: PreferencePatch[] = [];
    door.tell((patch) => told.push(patch));
    const appearance = { ...get(door.held).appearance, body: 15 };
    door.setLang("zh");
    door.setWelcomed(true);
    door.setPanel(false);
    door.setAppearance(appearance);
    door.setProxying("never");
    door.setChord("go.city", "accel+2");
    door.setTier("zen");
    door.setTheme({ tokens: { "--color-accent": "#3366ff" }, css: ".x {}" });

    expect(told).toEqual([
      { lang: "zh" },
      { welcomed: true },
      { panel: false },
      {
        appearance: {
          lighting: appearance.lighting,
          sans: appearance.sans,
          mono: appearance.mono,
          sans_stack: appearance.sansStack,
          mono_stack: appearance.monoStack,
          body_px: 15,
          density: appearance.density,
          chroma: appearance.chroma,
          motion: appearance.motion,
          glass: appearance.glass,
          blend_percent: appearance.blend,
        },
      },
      { proxying: "never" },
      { chord: { action: "go.city", spelled: "accel+2" } },
      { theme: { tokens: { "--color-accent": "#3366ff" }, css: ".x {}" } },
    ]);
  });
});
