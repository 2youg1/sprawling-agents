// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


import { get } from "svelte/store";

import { QUERIES } from "./asking";
import { putPreferences } from "./commands";
import { blendOf, FAMILY_SHAPE, type Appearance } from "./appearance";
import type { PreferenceDoor, Preferences } from "./prefs";
import { themeOf } from "./theme_override";
import type { Connection } from "./socket";
import type { Appearance as WireAppearance, PreferencesAnswer } from "../wire";


export type Keeper =
  
  
  | "browser"
  
  
  | "city";

export function keepWithCity(door: PreferenceDoor, conn: Connection): void {
  door.tell((patch) => {
    if (get(conn.state).kind === "live") conn.command(putPreferences(patch));
  });
  conn.asking.ask(QUERIES.preferences).subscribe((answer) => {
    if (answer === undefined || !("preferences" in answer)) return;
    door.adopt(adopted(get(door.held), answer.preferences), answer.preferences.chords ?? []);
  });
}


export function adopted(held: Preferences, answer: PreferencesAnswer): Preferences {
  return {
    ...held,
    lang: answer.lang ?? held.lang,
    welcomed: answer.welcomed ?? held.welcomed,
    panel: answer.panel ?? held.panel,
    proxying: answer.proxying ?? held.proxying,
    appearance:
      answer.appearance === undefined
        ? held.appearance
        : appearanceOfCity(answer.appearance, held.appearance),
    theme: answer.theme === undefined ? held.theme : themeOf(answer.theme),
  };
}

export function appearanceOnWire(next: Appearance): WireAppearance {
  return {
    lighting: next.lighting,
    sans: next.sans,
    mono: next.mono,
    sans_stack: next.sansStack,
    mono_stack: next.monoStack,
    reading: next.reading,
    fallbacks: [...next.fallbacks],
    features: [...next.features],
    body_px: next.body,
    line_height: next.lineHeight,
    letter_spacing: next.tracking,
    weight: next.weight,
    density: next.density,
    chroma: next.chroma,
    motion: next.motion,
    glass: next.glass,
    blend_percent: next.blend,
  };
}


function appearanceOfCity(stated: WireAppearance, held: Appearance): Appearance {
  return {
    lighting: stated.lighting,
    sans: stated.sans,
    mono: stated.mono,
    sansStack: stated.sans_stack,
    monoStack: stated.mono_stack,
    reading: stated.reading ?? held.reading,
    
    
    fallbacks:
      stated.fallbacks === undefined
        ? held.fallbacks
        : stated.fallbacks.filter((name) => FAMILY_SHAPE.test(name)).slice(0, 2),
    features: stated.features ?? held.features,
    body: stated.body_px ?? null,
    lineHeight: stated.line_height === undefined ? held.lineHeight : stated.line_height,
    tracking: stated.letter_spacing === undefined ? held.tracking : stated.letter_spacing,
    weight: stated.weight ?? held.weight,
    density: stated.density,
    chroma: stated.chroma,
    motion: stated.motion,
    glass: stated.glass ?? held.glass,
    blend:
      stated.blend_percent === undefined ? held.blend : blendOf(String(stated.blend_percent ?? "")),
  };
}
