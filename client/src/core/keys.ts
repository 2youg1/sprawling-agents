// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Every key the shell listens for, in one table.
//
// A key was three facts spread over three files: the shell matched a
// letter, the rail drew a string, and nothing could change either. Here
// an action names what a key does, a chord says which keys reach it,
// the person's own chord replaces the default, and one function answers
// "which action was that". The rail, the shell and the settings section
// read this table; none of them spells a key itself.
//
// **A chord is what every other application on the machine means by
// one**: the accelerator (Cmd on a Mac, Ctrl everywhere else), Shift,
// and the key itself. The `g`-then-letter prefix this table shipped
// with is gone (roadmap 3.9). It was a habit borrowed from one text
// editor rather than from the platform, it held a `g` typed anywhere
// outside a text box for a second and a half before deciding it meant
// nothing, and no other window the person has open answers it.
//
// **Shift is a fact only beside the accelerator.** With no modifier
// held, the browser hands over the character the layout produced, and
// that character already says whether Shift was down: `?` is `?`
// however a keyboard types it. With the accelerator held, the chord
// has to name Shift itself, or `Ctrl+Shift+A` and `Ctrl+A` would be
// one key.
//
// The person's overrides are kept with the rest of their preferences,
// one row per action, and this file asks `prefs.ts` for the chord of
// an action rather than naming the row: what a row is called is that
// file's single decision, and so is what a browser with no storage
// gets instead. `spell` below is already the written form the person's
// own `[ui.keys]` table will hold when roadmap 3.1 moves these rows
// out of the browser.

import { createSignal } from "solid-js";
import type { Accessor } from "solid-js";

import type { Key } from "./lang";
import { preferences } from "./prefs";
import type { PreferenceDoor } from "./prefs";

// What a key can do. `go.*` moves the address bar, the rest act on the
// shell itself.
export const ACTIONS = [
  "go.talk",
  "go.city",
  "go.mcp",
  "go.record",
  "go.cost",
  "go.registry",
  "go.setup",
  "go.waiting",
  "palette",
  "rail.toggle",
  "help",
  "composer.focus",
  "run.stop",
] as const;

export type Action = (typeof ACTIONS)[number];

// Which modifier this machine calls the accelerator, and therefore
// which glyph a chord is drawn with.
export type Platform = "mac" | "other";

export interface Chord {
  // Cmd on a Mac, Ctrl elsewhere. One fact, drawn two ways, so a chord
  // a person set on one machine still reads correctly on another.
  readonly accel: boolean;
  // Held beside the accelerator. False on every chord that holds no
  // accelerator, because the character such a press produces already
  // says whether Shift was down.
  readonly shift: boolean;
  // What `KeyboardEvent.key` holds, in the case the person types it.
  readonly key: string;
}

const ACCEL_MARK = "accel+";
const SHIFT_MARK = "shift+";

// A key is stored and matched without case, and this is the one place
// that decides what "without case" means.
function folded(key: string): string {
  return key.toLowerCase();
}

function plain(key: string): Chord {
  return { accel: false, shift: false, key };
}
function accel(key: string): Chord {
  return { accel: true, shift: false, key };
}
function accelShift(key: string): Chord {
  return { accel: true, shift: true, key };
}

// The chords this client ships with.
//
// **Six pages sit on the six digits, in the order the rail draws
// them**, with settings taken out: settings is on the comma that every
// browser and every editor puts it on, which leaves the sixth digit
// for the registry - the one screen that reached this build with no
// way in at all.
//
// `?` and `/` hold no modifier because the shell reads them only
// outside a text box; that rule is `app.tsx`'s, and it is the reason
// those two can stay single keys.
export const DEFAULTS: Readonly<Record<Action, Chord>> = {
  "go.talk": accel("1"),
  "go.city": accel("2"),
  "go.mcp": accel("3"),
  "go.record": accel("4"),
  "go.cost": accel("5"),
  "go.registry": accel("6"),
  "go.setup": accel(","),
  "go.waiting": accelShift("a"),
  palette: accel("k"),
  "rail.toggle": accel("b"),
  help: plain("?"),
  "composer.focus": plain("/"),
  "run.stop": accel("."),
};

// The word each action is called by, which is a phrase key rather than
// a phrase: this file holds no words.
export const LABELS: Readonly<Record<Action, Key>> = {
  "go.talk": "nav_mayor",
  "go.city": "nav_city",
  "go.mcp": "nav_mcp",
  "go.record": "nav_the_record",
  "go.cost": "cost_title",
  "go.registry": "nav_registry",
  "go.setup": "nav_settings",
  "go.waiting": "wait_title",
  palette: "nav_palette",
  "rail.toggle": "keys_rail",
  help: "keys_help",
  "composer.focus": "keys_composer",
  "run.stop": "city_stop",
};

// The keys a browser keeps for itself beside the accelerator: it
// closes a tab, opens a tab and opens a window before the page is
// told anything, with or without Shift. A chord bound to one of them
// therefore never fires. It is shown to the person rather than
// refused, for the same reason a collision is.
const RESERVED: readonly string[] = ["n", "t", "w"];

export function reserved(held: Chord): boolean {
  return held.accel && RESERVED.includes(folded(held.key));
}

// ------------------------------------------------------------- spelling

// The one written form of a chord: what is stored, and what a chord
// read back from storage is compared against.
export function spell(held: Chord): string {
  const accelMark = held.accel ? ACCEL_MARK : "";
  const shiftMark = held.shift ? SHIFT_MARK : "";
  return `${accelMark}${shiftMark}${folded(held.key)}`;
}

// The chord a written form names, or none when the text is not one.
// Storage is the only writer, but a row a person edited by hand is
// still a row this has to answer for.
export function readChord(text: string): Chord | null {
  const written = text.trim();
  const accel = written.startsWith(ACCEL_MARK);
  const afterAccel = accel ? written.slice(ACCEL_MARK.length) : written;
  const shift = afterAccel.startsWith(SHIFT_MARK);
  const key = shift ? afterAccel.slice(SHIFT_MARK.length) : afterAccel;
  // Shift without the accelerator is a chord nothing here can match,
  // because a press holding no accelerator is judged by the character
  // it produced rather than by the modifiers that produced it.
  if (key === "" || /\s/.test(key) || (shift && !accel)) {
    return null;
  }
  return { accel, shift, key: folded(key) };
}

// ------------------------------------------------------------ rendering

// Keys whose own name is longer than the mark a person expects to see.
const FACES: Readonly<Record<string, string>> = {
  " ": "Space",
  escape: "Esc",
  enter: "Enter",
  tab: "Tab",
  arrowup: "↑",
  arrowdown: "↓",
  arrowleft: "←",
  arrowright: "→",
};

function face(key: string): string {
  const named = FACES[folded(key)];
  if (named !== undefined) {
    return named;
  }
  return key.length === 1 ? key.toUpperCase() : key;
}

// The marks a chord is drawn as, in the order they are pressed, each
// the way this platform writes it.
export function marks(held: Chord, platform: Platform): readonly string[] {
  const mac = platform === "mac";
  const out: string[] = [];
  if (held.accel) {
    out.push(mac ? "⌘" : "Ctrl");
  }
  if (held.shift) {
    out.push(mac ? "⇧" : "Shift");
  }
  out.push(face(held.key));
  return out;
}

// Which modifier this browser's machine calls the accelerator.
// `navigator.platform` is what everybody used and what every engine
// deprecated, so the user agent string answers instead.
export function platformOf(userAgent: string): Platform {
  return /mac|iphone|ipad|ipod/i.test(userAgent) ? "mac" : "other";
}

// ------------------------------------------------------------- matching

// The part of a key press this table judges. A `KeyboardEvent` is one;
// so is the record a test writes.
export interface Pressed {
  readonly key: string;
  readonly ctrlKey: boolean;
  readonly metaKey: boolean;
  readonly shiftKey: boolean;
  readonly altKey: boolean;
}

export function matches(held: Chord, pressed: Pressed): boolean {
  if (pressed.altKey) {
    return false;
  }
  if (held.accel !== (pressed.ctrlKey || pressed.metaKey)) {
    return false;
  }
  // Shift is asked about only beside the accelerator; the file's
  // opening paragraph says why the other case must not ask.
  if (held.accel && held.shift !== pressed.shiftKey) {
    return false;
  }
  return folded(held.key) === folded(pressed.key);
}

export interface Conflict {
  readonly spelled: string;
  readonly actions: readonly Action[];
}

// Every chord that two or more actions claim. A conflict is shown, not
// prevented: the person deciding which of the two they meant needs to
// see both, and a binding that silently refuses to take teaches
// nothing.
export function conflictsOf(bound: Readonly<Record<Action, Chord>>): readonly Conflict[] {
  const byChord = new Map<string, Action[]>();
  for (const action of ACTIONS) {
    const spelled = spell(bound[action]);
    const held = byChord.get(spelled);
    if (held === undefined) {
      byChord.set(spelled, [action]);
    } else {
      held.push(action);
    }
  }
  const out: Conflict[] = [];
  for (const [spelled, actions] of byChord) {
    if (actions.length > 1) {
      out.push({ spelled, actions });
    }
  }
  return out;
}

// -------------------------------------------------------------- the map

export interface Keymap {
  readonly platform: Platform;
  readonly bound: Accessor<Readonly<Record<Action, Chord>>>;
  readonly chord: (action: Action) => Chord;
  readonly bind: (action: Action, chord: Chord) => void;
  // Back to what this client ships with.
  readonly reset: (action: Action) => void;
  readonly resetAll: () => void;
  readonly changed: (action: Action) => boolean;
  readonly conflicts: Accessor<readonly Conflict[]>;
  // Which action a key press reaches.
  readonly acting: (pressed: Pressed) => Action | null;
}

function stored(kept: PreferenceDoor): Record<Action, Chord> {
  const out: Record<string, Chord> = {};
  for (const action of ACTIONS) {
    out[action] = readChord(kept.chord(action)) ?? DEFAULTS[action];
  }
  // Built from the same list the type is built from, so every action
  // has a chord; the fallback keeps that true for a reader who cannot
  // see the loop.
  return { ...DEFAULTS, ...out };
}

export function loadKeys(kept: PreferenceDoor, userAgent: string): Keymap {
  const platform = platformOf(userAgent);
  const [bound, setBound] = createSignal<Readonly<Record<Action, Chord>>>(stored(kept));
  const put = (action: Action, chord: Chord) => {
    setBound((held) => ({ ...held, [action]: chord }));
  };
  return {
    platform,
    bound,
    chord: (action) => bound()[action],
    bind(action, chord) {
      kept.setChord(action, spell(chord));
      put(action, chord);
    },
    reset(action) {
      kept.setChord(action, "");
      put(action, DEFAULTS[action]);
    },
    resetAll() {
      for (const action of ACTIONS) {
        kept.setChord(action, "");
      }
      setBound({ ...DEFAULTS });
    },
    changed: (action) => spell(bound()[action]) !== spell(DEFAULTS[action]),
    conflicts: () => conflictsOf(bound()),
    acting(pressed) {
      const held = bound();
      return ACTIONS.find((action) => matches(held[action], pressed)) ?? null;
    },
  };
}

// The map this page runs on. One per document, because the shell that
// listens and the settings section that rebinds have to be looking at
// the same table; a second copy would let a rebind go unheard.
let shared: Keymap | undefined;

export function keymap(): Keymap {
  shared ??= loadKeys(preferences(), typeof navigator === "undefined" ? "" : navigator.userAgent);
  return shared;
}
