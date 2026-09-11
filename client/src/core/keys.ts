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
// A chord is at most three facts: the accelerator (Cmd on a Mac, Ctrl
// everywhere else), a prefix key pressed and released first (`g`), and
// the key itself. Nothing here needs Shift as a fact of its own: the
// browser already hands `?` and `$` as the character that was typed.
//
// The person's overrides live in the same `localStorage` the rest of
// their preferences do, one row per action, so reading a stored chord
// never parses a document and never fails in a way that needs handling.

import { createSignal } from "solid-js";
import type { Accessor } from "solid-js";

import type { Key } from "./lang";

// What a key can do. `go.*` moves the address bar, the rest act on the
// shell itself.
export const ACTIONS = [
  "go.talk",
  "go.city",
  "go.mcp",
  "go.record",
  "go.cost",
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
  // A key pressed and released before the chord's own key, or none.
  readonly prefix: string | null;
  // What `KeyboardEvent.key` holds, in the case the person types it.
  readonly key: string;
}

const PREFIX = "g";
const ACCEL_MARK = "accel+";
const ROW = "sprawling.key.";

function plain(key: string): Chord {
  return { accel: false, prefix: null, key };
}
function accel(key: string): Chord {
  return { accel: true, prefix: null, key };
}
function after(key: string): Chord {
  return { accel: false, prefix: PREFIX, key };
}

// The chords this client ships with. `g $` and `?` both need Shift on a
// US keyboard; both stay, because a layout that cannot type them can
// now be given a chord that it can.
export const DEFAULTS: Readonly<Record<Action, Chord>> = {
  "go.talk": after("m"),
  "go.city": after("c"),
  "go.mcp": after("x"),
  "go.record": after("r"),
  "go.cost": after("$"),
  "go.setup": after("s"),
  "go.waiting": after("w"),
  palette: accel("k"),
  "rail.toggle": plain("["),
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
  "go.setup": "nav_settings",
  "go.waiting": "wait_title",
  palette: "nav_palette",
  "rail.toggle": "keys_rail",
  help: "keys_help",
  "composer.focus": "keys_composer",
  "run.stop": "city_stop",
};

// How long a prefix waits for its second key before it is forgotten.
export const PREFIX_MS = 1500;

// ------------------------------------------------------------- spelling

// The one written form of a chord: what is stored, and what a chord
// read back from storage is compared against.
export function spell(held: Chord): string {
  const base = held.accel ? `${ACCEL_MARK}${held.key}` : held.key;
  return held.prefix === null ? base : `${held.prefix} ${base}`;
}

// The chord a written form names, or none when the text is not one.
// Storage is the only writer, but a row a person edited by hand is
// still a row this has to answer for.
export function readChord(text: string): Chord | null {
  const words = text.trim().split(" ");
  if (words.length > 2) {
    return null;
  }
  const last = words.at(-1);
  const first = words.length === 2 ? words.at(0) : undefined;
  if (last === undefined || last === "") {
    return null;
  }
  if (first !== undefined && first.length !== 1) {
    return null;
  }
  const marked = last.startsWith(ACCEL_MARK);
  const key = marked ? last.slice(ACCEL_MARK.length) : last;
  if (key === "" || key.includes(" ")) {
    return null;
  }
  // An accelerator after a prefix is a chord nothing can type without
  // the two halves fighting over the modifier.
  if (marked && first !== undefined) {
    return null;
  }
  return { accel: marked, prefix: first ?? null, key };
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
  const named = FACES[key.toLowerCase()];
  if (named !== undefined) {
    return named;
  }
  return key.length === 1 ? key.toUpperCase() : key;
}

// The marks a chord is drawn as, in the order they are pressed. A
// prefixed chord keeps the case a person types it in, because `g c` is
// two keystrokes and reads as two letters; an accelerated one is drawn
// the way its platform writes it.
export function marks(held: Chord, platform: Platform): readonly string[] {
  if (held.prefix !== null) {
    return [held.prefix, held.key];
  }
  const mark = platform === "mac" ? "⌘" : "Ctrl";
  return held.accel ? [mark, face(held.key)] : [face(held.key)];
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
  readonly altKey: boolean;
}

export function matches(held: Chord, pressed: Pressed, prefix: string | null): boolean {
  if (held.prefix !== prefix || pressed.altKey) {
    return false;
  }
  if (held.accel !== (pressed.ctrlKey || pressed.metaKey)) {
    return false;
  }
  return held.key.toLowerCase() === pressed.key.toLowerCase();
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

// The little of `Storage` this needs, so a test hands it a map and the
// browser hands it `localStorage`.
export interface Rows {
  getItem: (key: string) => string | null;
  setItem: (key: string, value: string) => void;
  removeItem: (key: string) => void;
}

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
  // The keys that open a chord rather than finishing one.
  readonly prefixes: Accessor<readonly string[]>;
  // Which action a key press reaches, given the prefix already held.
  readonly acting: (pressed: Pressed, prefix: string | null) => Action | null;
}

function stored(rows: Rows): Record<Action, Chord> {
  const out: Record<string, Chord> = {};
  for (const action of ACTIONS) {
    out[action] = readChord(rows.getItem(ROW + action) ?? "") ?? DEFAULTS[action];
  }
  // Built from the same list the type is built from, so every action
  // has a chord; the fallback keeps that true for a reader who cannot
  // see the loop.
  return { ...DEFAULTS, ...out };
}

export function loadKeys(rows: Rows, userAgent: string): Keymap {
  const platform = platformOf(userAgent);
  const [bound, setBound] = createSignal<Readonly<Record<Action, Chord>>>(stored(rows));
  const put = (action: Action, chord: Chord) => {
    setBound((held) => ({ ...held, [action]: chord }));
  };
  return {
    platform,
    bound,
    chord: (action) => bound()[action],
    bind(action, chord) {
      rows.setItem(ROW + action, spell(chord));
      put(action, chord);
    },
    reset(action) {
      rows.removeItem(ROW + action);
      put(action, DEFAULTS[action]);
    },
    resetAll() {
      for (const action of ACTIONS) {
        rows.removeItem(ROW + action);
      }
      setBound({ ...DEFAULTS });
    },
    changed: (action) => spell(bound()[action]) !== spell(DEFAULTS[action]),
    conflicts: () => conflictsOf(bound()),
    prefixes() {
      const held = bound();
      const out = new Set<string>();
      for (const action of ACTIONS) {
        const prefix = held[action].prefix;
        if (prefix !== null) {
          out.add(prefix);
        }
      }
      return [...out];
    },
    acting(pressed, prefix) {
      const held = bound();
      return ACTIONS.find((action) => matches(held[action], pressed, prefix)) ?? null;
    },
  };
}

// The map this page runs on. One per document, because the shell that
// listens and the settings section that rebinds have to be looking at
// the same table; a second copy would let a rebind go unheard.
let shared: Keymap | undefined;

export function keymap(): Keymap {
  shared ??= loadKeys(
    typeof localStorage === "undefined" ? memory() : localStorage,
    typeof navigator === "undefined" ? "" : navigator.userAgent,
  );
  return shared;
}

// What a browser without storage remembers: this session, and no
// longer. A rebind still takes effect; it just does not outlive the
// tab.
function memory(): Rows {
  const held = new Map<string, string>();
  return {
    getItem: (key) => held.get(key) ?? null,
    setItem: (key, value) => {
      held.set(key, value);
    },
    removeItem: (key) => {
      held.delete(key);
    },
  };
}
