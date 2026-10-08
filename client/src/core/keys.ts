// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Every key the shell listens for, in one table.
//
// A key was three facts spread over three files: the shell matched a
// letter, a view drew a string, and nothing could change either. Here
// an action names what a key does, a chord says which keys reach it,
// the person's own chord replaces the default, and one function answers
// "which action was that". The edge keys, the shell and the settings
// section read this table; none of them spells a key itself. Walking
// the lines of a list is `lines.ts`'s table, and a press is `press.ts`'s.
//
// **A chord is what every other application on the machine means by
// one**: the accelerator (Cmd on a Mac, Ctrl everywhere else), Shift,
// and the key itself. The `g`-then-letter prefix this table shipped
// with is gone (roadmap 3.9). It was a habit borrowed from one text
// editor rather than from the platform, it held a `g` typed anywhere
// outside a text box for a second and a half before deciding it meant
// nothing, and no other window the person has open answers it.
//
// **An action that changes state or opens a layer holds the
// accelerator.** A single key is what a person types, and a person
// types into the wrong place: a letter pressed while the focus sat on a
// message, a card or the page rather than in the box would fork a
// conversation, answer a decide card, change the tier or open a sheet.
// Only `composer.focus` stays a single key, because all `/` does is
// move the focus into the message box, and a `/` pressed by mistake
// leaves the next letters typed there, which is where they were meant
// to go.
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

import { derived, get, writable } from "svelte/store";
import type { Readable } from "svelte/store";

import type { Key } from "./lang";
import { preferences } from "./prefs";
import type { PreferenceDoor } from "./prefs";
import type { Pressed } from "./press";

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
  "tier.cycle",
  "mailbox",
  "inspect",
  "help",
  "composer.focus",
  "run.stop",
  "fork.here",
  "decide.yes",
  "decide.edit",
  "decide.no",
  "finder",
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
export function folded(key: string): string {
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

// What an action is reached by: a chord, or `null` for an action no key
// reaches until the person binds one in settings. An unbound action
// keeps every other way in it had - its page in the settings tree, its
// verb in the palette and after `/`, its button.
export type Bound = Chord | null;

// The chords this client ships with.
//
// **No default sits on a chord a browser gives a function of its
// own** (`BROWSER_KEEPS` below, the audit of the four browsers' official
// shortcut lists). A page can often take such a chord first, and that
// is the harm: the person who pressed Accel-P to print, Accel-J for
// downloads or Accel-1 for the first tab got the city's page instead.
// So the six page digits, settings on the comma, the waiting list, the
// mailbox, the right pane, the stop, the branch, the three decide
// letters and the file finder ship with no chord; each keeps the route
// it already had, and the person may bind any of them.
//
// The palette moved from K, which three of the four browsers give to
// their search box, to the slash beside the accelerator, which none of
// the four lists; it is the one door to every page and every verb, so
// it keeps a chord. The key list, which used to sit there, is the keys
// section of settings and has no chord of its own. The tier stays on
// the backslash, which no browser lists outside a PDF viewer.
//
// `/` alone holds no modifier; the paragraph at the top of this file
// says why, and `matches` below keeps it out of a text box. Firefox's
// Quick Find answers `/` too, but only outside a text field, and its
// find bar keeps Accel-F.
export const DEFAULTS: Readonly<Record<Action, Bound>> = {
  "go.talk": null,
  "go.city": null,
  "go.mcp": null,
  "go.record": null,
  "go.cost": null,
  "go.registry": null,
  "go.setup": null,
  "go.waiting": null,
  palette: accel("/"),
  "tier.cycle": accel("\\"),
  mailbox: null,
  inspect: null,
  help: null,
  "composer.focus": plain("/"),
  "run.stop": null,
  "fork.here": null,
  "decide.yes": null,
  "decide.edit": null,
  "decide.no": null,
  finder: null,
};

// The chords a browser gives a function of its own, from the official
// shortcut lists of Chrome (support.google.com/chrome/answer/157179),
// Edge (support.microsoft.com, "Keyboard shortcuts in Microsoft Edge"),
// Firefox (the `<key>` table of `browser/base/content/browser-sets.inc.xhtml`,
// which its support page is written from) and Safari
// (support.apple.com/guide/safari/cpsh003). No default may sit on one;
// a person may still bind one, because a person who never prints may
// want Accel-P for the finder.
const BROWSER_KEEPS: readonly Chord[] = [
  // Select a tab: all four.
  ...["1", "2", "3", "4", "5", "6", "7", "8", "9"].map(accel),
  // Settings on macOS: Chrome, Firefox, Safari.
  accel(","),
  // Search from the address bar: Chrome, Edge, Firefox.
  accel("k"),
  // Downloads: Chrome, Edge, Firefox.
  accel("j"),
  // Bookmarks sidebar: Firefox.
  accel("b"),
  // Print: all four.
  accel("p"),
  // Stop loading on macOS: Firefox, Safari.
  accel("."),
  // Search open tabs: Firefox; add-ons: Firefox on Windows and Linux,
  // and on macOS under E; search in the sidebar: Edge; Collections:
  // Edge; switch text direction: Firefox.
  accelShift("a"),
  accelShift("f"),
  accelShift("e"),
  accelShift("y"),
  accelShift("x"),
];

// Whether a browser gives this chord a function of its own.
export function browserKeeps(held: Bound): boolean {
  return held !== null && BROWSER_KEEPS.some((kept) => spell(kept) === spell(held));
}

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
  "tier.cycle": "edge_layers",
  mailbox: "edge_mailbox",
  inspect: "keys_inspect",
  help: "keys_help",
  "composer.focus": "keys_composer",
  "run.stop": "slash_stop",
  "fork.here": "fork_here",
  "decide.yes": "keys_decide_yes",
  "decide.edit": "keys_decide_edit",
  "decide.no": "keys_decide_no",
  finder: "keys_finder",
};

// The keys a browser keeps for itself beside the accelerator: it
// closes a tab, opens a tab and opens a window before the page is
// told anything, with or without Shift. A chord bound to one of them
// therefore never fires. It is shown to the person rather than
// refused, for the same reason a collision is.
const RESERVED: readonly string[] = ["n", "t", "w"];

export function reserved(held: Bound): boolean {
  return held !== null && held.accel && RESERVED.includes(folded(held.key));
}

// ------------------------------------------------------------- spelling

// The one written form of a chord: what is stored, and what a chord
// read back from storage is compared against. An unbound action is
// spelled as nothing.
export function spell(held: Bound): string {
  if (held === null) return "";
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

export function face(key: string): string {
  const named = FACES[folded(key)];
  if (named !== undefined) {
    return named;
  }
  return key.length === 1 ? key.toUpperCase() : key;
}

// The marks a chord is drawn as, in the order they are pressed, each
// the way this platform writes it; none for an unbound action.
export function marks(held: Bound, platform: Platform): readonly string[] {
  if (held === null) return [];
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

export function matches(held: Bound, pressed: Pressed): boolean {
  if (held === null || pressed.altKey) {
    return false;
  }
  if (held.accel !== (pressed.ctrlKey || pressed.metaKey)) {
    return false;
  }
  // Inside a text field a single key is what the person is typing, so
  // only a chord holding the accelerator is the shell's. Without this
  // rule a `/` in a sentence moved the focus to the composer and a
  // backslash changed the tier mid-word.
  if (pressed.target === "field" && !held.accel) {
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
export function conflictsOf(bound: Readonly<Record<Action, Bound>>): readonly Conflict[] {
  const byChord = new Map<string, Action[]>();
  for (const action of ACTIONS) {
    const spelled = spell(bound[action]);
    if (spelled === "") continue;
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
  readonly bound: Readable<Readonly<Record<Action, Bound>>>;
  readonly chord: (action: Action) => Bound;
  readonly bind: (action: Action, chord: Chord) => void;
  // Back to what this client ships with.
  readonly reset: (action: Action) => void;
  readonly resetAll: () => void;
  readonly changed: (action: Action) => boolean;
  readonly conflicts: Readable<readonly Conflict[]>;
  // Which action a key press reaches.
  readonly acting: (pressed: Pressed) => Action | null;
}

function stored(kept: PreferenceDoor): Record<Action, Bound> {
  const out: Record<string, Bound> = {};
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
  const held = writable<Readonly<Record<Action, Bound>>>(stored(kept));
  const put = (action: Action, chord: Bound) => {
    held.update((was) => ({ ...was, [action]: chord }));
  };
  return {
    platform,
    bound: held,
    chord: (action) => get(held)[action],
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
      held.set({ ...DEFAULTS });
    },
    changed: (action) => spell(get(held)[action]) !== spell(DEFAULTS[action]),
    conflicts: derived(held, conflictsOf),
    acting(pressed) {
      const bound = get(held);
      return ACTIONS.find((action) => matches(bound[action], pressed)) ?? null;
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
