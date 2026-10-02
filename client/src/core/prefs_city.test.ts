// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";
import { get } from "svelte/store";

import { loadPreferences } from "./prefs";
import { adopted } from "./prefs_city";
import { memory } from "./rows";
import type { PreferencePatch } from "../wire";

describe("the preferences the city keeps", () => {
  test("the city's answer is taken over what this browser held", () => {
    const held = get(loadPreferences(memory(), "en").held);
    const answer = {
      lang: "zh",
      panel: false,
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
        glass: "on",
        blend: null,
      },
    });
  });

  test("each named change is told to the city as its own patch", () => {
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
        },
      },
      { proxying: "never" },
      { chord: { action: "go.city", spelled: "accel+2" } },
    ]);
  });
});
