// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { derived, get, writable } from "./store";
import type { Readable } from "./store";

import type { Key } from "./lang";
import { preferences } from "./prefs";
import type { PreferenceDoor } from "./prefs";
import type { Pressed } from "./press";

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
  "session.switch",
  "tier.cycle",
  "workspace.navigation",
  "workspace.subject",
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

export type Platform = "mac" | "other";

export interface Chord {
  readonly accel: boolean;
  readonly shift: boolean;
  readonly key: string;
}

const ACCEL_MARK = "accel+";
const SHIFT_MARK = "shift+";

export function folded(key: string): string {
  return key.toLowerCase();
}

function plain(key: string): Chord {
  return { accel: false, shift: false, key };
}
export function accel(key: string): Chord {
  return { accel: true, shift: false, key };
}
export function accelShift(key: string): Chord {
  return { accel: true, shift: true, key };
}

export type Bound = Chord | null;

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
  "session.switch": accelShift("u"),
  "tier.cycle": accel("\\"),
  "workspace.navigation": null,
  "workspace.subject": null,
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
  "session.switch": "world_session_switcher",
  "tier.cycle": "appearance_tier",
  "workspace.navigation": "shell_navigation",
  "workspace.subject": "shell_file_mode",
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

const RESERVED: readonly string[] = ["n", "t", "w"];

export function reserved(held: Bound): boolean {
  return held !== null && held.accel && RESERVED.includes(folded(held.key));
}

export function spell(held: Bound): string {
  if (held === null) return "";
  const accelMark = held.accel ? ACCEL_MARK : "";
  const shiftMark = held.shift ? SHIFT_MARK : "";
  return `${accelMark}${shiftMark}${folded(held.key)}`;
}

export function readChord(text: string): Chord | null {
  const written = text.trim();
  const accel = written.startsWith(ACCEL_MARK);
  const afterAccel = accel ? written.slice(ACCEL_MARK.length) : written;
  const shift = afterAccel.startsWith(SHIFT_MARK);
  const key = shift ? afterAccel.slice(SHIFT_MARK.length) : afterAccel;
  if (key === "" || /\s/.test(key) || (shift && !accel)) {
    return null;
  }
  return { accel, shift, key: folded(key) };
}

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

export function platformOf(userAgent: string): Platform {
  return /mac|iphone|ipad|ipod/i.test(userAgent) ? "mac" : "other";
}

export function matches(held: Bound, pressed: Pressed): boolean {
  if (held === null || pressed.altKey) {
    return false;
  }
  if (held.accel !== (pressed.ctrlKey || pressed.metaKey)) {
    return false;
  }
  if (pressed.target === "field" && !held.accel) {
    return false;
  }
  if (held.accel && held.shift !== pressed.shiftKey) {
    return false;
  }
  return folded(held.key) === folded(pressed.key);
}

export interface Conflict {
  readonly spelled: string;
  readonly actions: readonly Action[];
}

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

export interface Keymap {
  readonly platform: Platform;
  readonly bound: Readable<Readonly<Record<Action, Bound>>>;
  readonly chord: (action: Action) => Bound;
  readonly bind: (action: Action, chord: Chord) => void;
  readonly reset: (action: Action) => void;
  readonly resetAll: () => void;
  readonly changed: (action: Action) => boolean;
  readonly conflicts: Readable<readonly Conflict[]>;
  readonly acting: (pressed: Pressed) => Action | null;
}

function stored(kept: PreferenceDoor): Record<Action, Bound> {
  const out: Record<string, Bound> = {};
  for (const action of ACTIONS) {
    out[action] = readChord(kept.chord(action)) ?? DEFAULTS[action];
  }
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

let shared: Keymap | undefined;

export function keymap(): Keymap {
  shared ??= loadKeys(preferences(), typeof navigator === "undefined" ? "" : navigator.userAgent);
  return shared;
}
