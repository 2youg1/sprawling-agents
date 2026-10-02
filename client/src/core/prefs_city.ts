// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The person's preferences as the city keeps them, in the person's own
// `~/.sprawling/config.toml`: the one place this client's record and
// the wire's record meet, and the one join between the door in
// `core/prefs.ts` and the connection.
//
// **The city wins, and only a live link tells it.** Every answer the
// city gives replaces what this browser held; a change made while the
// link is down stays in this browser, which `keeper` already says, and
// the next answer after the link returns is the one that counts.

import { get } from "svelte/store";

import { QUERIES } from "./asking";
import { putPreferences } from "./commands";
import { blendOf, type Appearance } from "./appearance";
import type { PreferenceDoor, Preferences } from "./prefs";
import type { Connection } from "./socket";
import type { Appearance as WireAppearance, PreferencesAnswer } from "../wire";

export function keepWithCity(door: PreferenceDoor, conn: Connection): void {
  door.tell((patch) => {
    if (get(conn.state).kind === "live") conn.command(putPreferences(patch));
  });
  conn.asking.ask(QUERIES.preferences).subscribe((answer) => {
    if (answer === undefined || !("preferences" in answer)) return;
    door.adopt(adopted(get(door.held), answer.preferences), answer.preferences.chords ?? []);
  });
}

// The city's answer taken over what this browser held. A field the
// answer leaves out, or states as never chosen, is one the person never
// settled with the city, so the browser's value stands for it.
export function adopted(held: Preferences, answer: PreferencesAnswer): Preferences {
  return {
    ...held,
    lang: answer.lang ?? held.lang,
    welcomed: answer.welcomed ?? held.welcomed,
    panel: answer.panel ?? held.panel,
    tier: answer.tier ?? held.tier,
    proxying: answer.proxying ?? held.proxying,
    appearance: answer.appearance === undefined ? held.appearance : appearanceOfCity(answer.appearance, held.appearance),
  };
}

export function appearanceOnWire(next: Appearance): WireAppearance {
  return {
    lighting: next.lighting,
    sans: next.sans,
    mono: next.mono,
    sans_stack: next.sansStack,
    mono_stack: next.monoStack,
    body_px: next.body,
    density: next.density,
    chroma: next.chroma,
    motion: next.motion,
    glass: next.glass,
    blend_percent: next.blend,
  };
}

// Glass and the opacity are absent from an answer by a city that keeps
// neither, and this browser's values stand. The city keeps the opacity
// as a bare percent and the slider's domain is this client's
// (`BLEND_PERCENT`), so a stated figure outside it - a file edited by
// hand - reads as no opacity, as a stored row would.
function appearanceOfCity(stated: WireAppearance, held: Appearance): Appearance {
  return {
    lighting: stated.lighting,
    sans: stated.sans,
    mono: stated.mono,
    sansStack: stated.sans_stack,
    monoStack: stated.mono_stack,
    body: stated.body_px ?? null,
    density: stated.density,
    chroma: stated.chroma,
    motion: stated.motion,
    glass: stated.glass ?? held.glass,
    blend: stated.blend_percent === undefined ? held.blend : blendOf(String(stated.blend_percent ?? "")),
  };
}
