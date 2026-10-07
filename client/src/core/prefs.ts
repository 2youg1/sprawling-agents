// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What this browser remembers about the person, as one value with one
// door in front of it.
//
// **Every row the client keeps is named here and nowhere else.** The
// settings screens, the shell and the keymap each used to spell their
// own row names against the same store, which is how a key written
// one way and read another goes unnoticed; now they read
// `PreferenceDoor` and this file is the only reader of `ROWS`.
//
// **Two layers keep these, and the city wins.** The person's own
// `~/.sprawling/config.toml` is the authority (roadmap 3.1) and this
// browser's store is the cache in front of it: the cache draws the
// first paint so no screen flashes the posture it ships with, and
// `adopt` replaces the whole record the moment the city answers. A
// change made before the city has answered is kept in this browser
// alone, and `keeper()` says so on the settings page rather than
// leaving a person to find out when they clear the browser's data.
//
// **A guess this build cannot read is dropped rather than repaired.**
// An unreadable row falls back to the posture the client ships with,
// which the theme already draws, so a cache can never become a
// second authority for a value somebody else owns.
//
// How hard the model thinks is deliberately absent. It is the city's
// `[model] effort`, and a copy kept here would ride on every dispatch
// from this browser and quietly overrule the city's own file.

import { derived, get, writable } from "svelte/store";
import type { Readable } from "svelte/store";

import { EDITORS, type Opening } from "./editor";
import { langOf, type Lang } from "./lang";
import { browserRows, type Rows } from "./rows";
import { sizingOf } from "./sizing";
import {
  Proxying,
  Tier as TierSchema,
  type Chord,
  type PreferencePatch,
  type Tier as WireTier,
} from "../wire";
import { appearanceOnWire, type Keeper } from "./prefs_city";
import { readTheme, type Theme } from "./theme_override";
import { CHROMAS, DENSITIES, FACES, GLASSES, LIGHTINGS, MOTIONS, STACK_SHAPE, blendOf, type Appearance } from "./appearance";
import type { Notifying } from "./notify";
import { SHOWINGS, type Showing } from "./results";
import { readWorkbench, spelledWorkbench, type Workbench } from "./workbench";

// ------------------------------------------------------------- the rows

// Every row this client keeps, by the name it is kept under. Two of
// them are families rather than single rows: a draft is kept per place
// a person writes, and a chord per action the person rebound, so those
// two are prefixes and the rest are whole names.
const ROWS = {
  lang: "sprawling.lang",
  welcomed: "sprawling.welcomed",
  // Whether the artefact panel beside a conversation is open.
  panel: "sprawling.talk.panel",
  lighting: "sprawling.appearance.lighting",
  sans: "sprawling.appearance.sans",
  mono: "sprawling.appearance.mono",
  sansStack: "sprawling.appearance.sans_stack",
  monoStack: "sprawling.appearance.mono_stack",
  body: "sprawling.appearance.body",
  density: "sprawling.appearance.density",
  chroma: "sprawling.appearance.chroma",
  motion: "sprawling.appearance.motion",
  glass: "sprawling.appearance.glass",
  blend: "sprawling.appearance.blend",
  // The colours laid over the built-in theme, as the wire spells them
  // (`crates/wire/spec/Preference.lean` D29), so the cache and the
  // city's record read through one grammar.
  theme: "sprawling.appearance.theme",
  // Which rule a provider attached from now on starts with. A person
  // behind a relay settles it once, on the network screen, instead of
  // on every form they open; an endpoint already attached keeps the
  // rule the city recorded for it.
  proxying: "sprawling.network.proxying",
  // Whether this browser raises a notification for an approval that
  // arrives while the window is away (client D7).
  notifying: "sprawling.notify",
  // Whether a room and the city open drawing the whole of what runs did
  // or only their results. The switch on either page writes it, so the
  // last choice is the default the next page opens with.
  showing: "sprawling.showing",
  // One unsent message per place a person writes, kept across a reload
  // or a page change; the rest of the name is the room or the run.
  draft: "sprawling.draft.",
  // One chord per action the person rebound; the rest of the name is
  // the action. An action left at its shipped chord has no row.
  chord: "sprawling.key.",
  editor: "sprawling.editor",
  cityFolder: "sprawling.editor.folder",
  // The panorama workbench's pane order and widths (client/Spec.lean §7K): a
  // fact of this screen, so it stays in this browser (client D24).
  workbench: "sprawling.workbench",
} as const;

// ------------------------------------------------------------ the shell

// How much of the world layer the page draws: none of it (`zen`), whole
// panels beside the conversation (`blend`), or all of it as the
// workspace with the conversation as a band along the bottom
// (`panorama`). Named for what is drawn rather than for a share of
// opacity, because the three are three layouts and not three points on
// one slider (docs/frontend-method.md §7H, client D17).
// Spelled by the wire (`wire::Tier`), in whose order the layers key
// cycles them: out from the conversation alone to the world, then back.
export type Tier = WireTier;
export const TIERS: readonly Tier[] = TierSchema.literals;

// The tier every launch opens in, whatever the last visit chose: the
// page is opened to talk (client/Spec.lean D47). The tier is held by
// this tab alone, so nothing stored or answered by the city brings an
// earlier one back.
const LAUNCH_TIER: Tier = "zen";

// Read out of the wire's own union rather than spelled again, so a rule
// the city adds is offered here without a second list to forget.
export const PROXYING_RULES: readonly Proxying[] = Proxying.members.map((rule) => rule.literal);

export const NOTIFYINGS: readonly Notifying[] = ["off", "on"];

// ---------------------------------------------------------- the reading

// The person's preferences, whole. One value rather than a dozen
// accessors because that is the shape the city will answer with, and
// because a screen that changes two of them at once must not be able
// to write one and drop the other.
export interface Preferences {
  readonly lang: Lang;
  // Whether this browser has walked through the welcome once. The city
  // decides whether setup is *needed*; this only decides whether a
  // person who skipped it is nagged again.
  readonly welcomed: boolean;
  readonly panel: boolean;
  readonly tier: Tier;
  readonly appearance: Appearance;
  readonly theme: Theme;
  readonly proxying: Proxying;
  readonly notifying: Notifying;
  readonly showing: Showing;
}

// The one way to the person's preferences: the record as it stands,
// who is keeping it, the city's answer coming the other way, named
// changes to it, and the families read by name because they have one
// row each per place, per action, or per screen.
//
// Named changes rather than one `write`, because each of them is its
// own `PutPreferences` patch: a caller that handed over a whole record
// would send the city every field to change one, and a caller that says
// which fact it is changing sends that fact. The bell and how a
// conversation is shown stay in this browser, since the city's record
// has no field for them; the workbench's order and widths stay here
// because they are a fact of this screen (client D24). `adopt` is the other
// direction and is therefore whole -
// an answer states every value at once, and a record applied field by
// field could be half of one answer and half of the last.
export interface PreferenceDoor {
  readonly held: Readable<Preferences>;
  readonly keeper: Readable<Keeper>;
  // The city's whole record, taken as the one that counts: it becomes
  // what `held` answers, it is mirrored into the cache so the next
  // first paint draws it rather than the shipped postures, and it is
  // the only thing that makes `keeper` say `city`. The chords it names
  // replace this browser's rows for those actions.
  readonly adopt: (stated: Preferences, chords: readonly Chord[]) => void;
  // Where each named change is told once it is made: the city's
  // `PutPreferences`, joined by `core/prefs_city.ts`. Until then a
  // change stays in this browser alone.
  readonly tell: (send: (patch: PreferencePatch) => void) => void;
  readonly setLang: (lang: Lang) => void;
  readonly setWelcomed: (done: boolean) => void;
  readonly setPanel: (open: boolean) => void;
  // Changes this tab's tier only: nothing stored, nothing told (D47).
  readonly setTier: (tier: Tier) => void;
  readonly setAppearance: (next: Appearance) => void;
  readonly setTheme: (next: Theme) => void;
  readonly setProxying: (rule: Proxying) => void;
  readonly setNotifying: (switched: Notifying) => void;
  readonly setShowing: (showing: Showing) => void;
  // The chord the person set for one action, or `""` for an action
  // they left alone. The spelling is the keymap's grammar, not this
  // file's: what is kept here is a name and a string.
  readonly chord: (action: string) => string;
  readonly setChord: (action: string, spelled: string) => void;
  // What was typed and not sent, by where it was typed. Not a signal:
  // the box that owns it reads it once when it mounts.
  readonly draft: (at: string) => string;
  readonly setDraft: (at: string, text: string) => void;
  // Whether that draft lives in this tab alone, the browser having refused it (4-63).
  readonly draftUnkept: (at: string) => Readable<boolean>;
  // Facts of the machine this browser runs on, never the city's (client/Spec.lean §4-39).
  readonly editor: () => Pick<Opening, "editor" | "folder">;
  readonly setEditor: (next: Pick<Opening, "editor" | "folder">) => void;
  readonly workbench: Readable<Workbench>;
  readonly setWorkbench: (next: Workbench) => void;
}

// A stored word, or the posture this client ships with when the row is
// empty or holds a word this build no longer offers.
function readOne<T extends string>(offered: readonly T[], raw: string | null, fallback: T): T {
  return offered.find((each) => each === raw) ?? fallback;
}

function readBody(raw: string | null): number | null {
  const said = sizingOf(raw ?? "");
  return said.kind === "sized" ? said.px : null;
}

function readStack(raw: string | null): string {
  return raw !== null && STACK_SHAPE.test(raw) ? raw : "";
}

function readLang(raw: string | null, fallback: string): Lang {
  return raw === "en" || raw === "zh" ? raw : langOf(fallback);
}

function readAppearance(rows: Rows): Appearance {
  return {
    lighting: readOne(LIGHTINGS, rows.getItem(ROWS.lighting), "system"),
    sans: readOne(FACES, rows.getItem(ROWS.sans), "geist"),
    mono: readOne(FACES, rows.getItem(ROWS.mono), "geist"),
    sansStack: readStack(rows.getItem(ROWS.sansStack)),
    monoStack: readStack(rows.getItem(ROWS.monoStack)),
    body: readBody(rows.getItem(ROWS.body)),
    density: readOne(DENSITIES, rows.getItem(ROWS.density), "comfortable"),
    chroma: readOne(CHROMAS, rows.getItem(ROWS.chroma), "full"),
    motion: readOne(MOTIONS, rows.getItem(ROWS.motion), "system"),
    glass: readOne(GLASSES, rows.getItem(ROWS.glass), "on"),
    blend: blendOf(rows.getItem(ROWS.blend) ?? ""),
  };
}

function writeAppearance(rows: Rows, next: Appearance): void {
  rows.setItem(ROWS.lighting, next.lighting);
  rows.setItem(ROWS.sans, next.sans);
  rows.setItem(ROWS.mono, next.mono);
  rows.setItem(ROWS.sansStack, next.sansStack);
  rows.setItem(ROWS.monoStack, next.monoStack);
  writeFigure(rows, ROWS.body, next.body);
  rows.setItem(ROWS.density, next.density);
  rows.setItem(ROWS.chroma, next.chroma);
  rows.setItem(ROWS.motion, next.motion);
  rows.setItem(ROWS.glass, next.glass);
  writeFigure(rows, ROWS.blend, next.blend);
}

// A size or an opacity the person has not stated is absent from storage
// too, so the stylesheet's own figure keeps its one home in the theme.
function writeFigure(rows: Rows, row: string, figure: number | null): void {
  if (figure === null) rows.removeItem(row);
  else rows.setItem(row, String(figure));
}

// The two words a yes-or-no row is written and read with. Each row is
// compared against one of them, because an absent `welcomed` is false
// (nobody walked the welcome yet) and an absent `panel` is open.
const YES = "yes";
const NO = "no";

function readPreferences(rows: Rows, browserLang: string): Preferences {
  return {
    lang: readLang(rows.getItem(ROWS.lang), browserLang),
    welcomed: rows.getItem(ROWS.welcomed) === YES,
    panel: rows.getItem(ROWS.panel) !== NO,
    tier: LAUNCH_TIER,
    appearance: readAppearance(rows),
    theme: readTheme(rows.getItem(ROWS.theme)),
    proxying: readOne(PROXYING_RULES, rows.getItem(ROWS.proxying), "except_local"),
    notifying: readOne(NOTIFYINGS, rows.getItem(ROWS.notifying), "off"),
    showing: readOne(SHOWINGS, rows.getItem(ROWS.showing), "whole"),
  };
}

// The whole record into the cache, the inverse of `readPreferences`:
// what the city last answered is what the next first paint draws.
function writePreferences(rows: Rows, next: Preferences): void {
  rows.setItem(ROWS.lang, next.lang);
  rows.setItem(ROWS.welcomed, next.welcomed ? YES : NO);
  rows.setItem(ROWS.panel, next.panel ? YES : NO);
  writeAppearance(rows, next.appearance);
  rows.setItem(ROWS.theme, JSON.stringify(next.theme));
  rows.setItem(ROWS.proxying, next.proxying);
  rows.setItem(ROWS.notifying, next.notifying);
  rows.setItem(ROWS.showing, next.showing);
}

// The door onto one store: a test hands it a map and a language tag,
// the page reaches the browser's through `preferences()`.
export function loadPreferences(rows: Rows, browserLang: string): PreferenceDoor {
  const held = writable<Preferences>(readPreferences(rows, browserLang));
  const keeper = writable<Keeper>("browser");
  const workbench = writable<Workbench>(readWorkbench(rows.getItem(ROWS.workbench)));
  // One write path for every kept change, the city's answer included: the
  // cache and the store move together, so a reader that redraws and a
  // reader that reloads the page never see two different records.
  const settle = (next: Preferences): void => {
    writePreferences(rows, next);
    held.set(next);
  };
  let told: (patch: PreferencePatch) => void = () => undefined;
  const writeChord = (action: string, spelled: string): void => {
    if (spelled === "") {
      rows.removeItem(ROWS.chord + action);
    } else {
      rows.setItem(ROWS.chord + action, spelled);
    }
  };
  return {
    held,
    keeper,
    adopt(stated, chords) {
      settle(stated);
      for (const each of chords) writeChord(each.action, each.spelled);
      keeper.set("city");
    },
    tell(send) {
      told = send;
    },
    setLang(lang) {
      settle({ ...get(held), lang });
      told({ lang });
    },
    setWelcomed(welcomed) {
      settle({ ...get(held), welcomed });
      told({ welcomed });
    },
    setPanel(panel) {
      settle({ ...get(held), panel });
      told({ panel });
    },
    setTier(tier) {
      held.set({ ...get(held), tier });
    },
    setAppearance(appearance) {
      settle({ ...get(held), appearance });
      told({ appearance: appearanceOnWire(appearance) });
    },
    setTheme(theme) {
      settle({ ...get(held), theme });
      told({ theme: { tokens: { ...theme.tokens }, css: theme.css } });
    },
    setProxying(proxying) {
      settle({ ...get(held), proxying });
      told({ proxying });
    },
    setNotifying(notifying) {
      settle({ ...get(held), notifying });
    },
    setShowing(showing) {
      settle({ ...get(held), showing });
    },
    chord: (action) => rows.getItem(ROWS.chord + action) ?? "",
    setChord(action, spelled) {
      writeChord(action, spelled);
      told({ chord: { action, spelled } });
    },
    draft: (at) => rows.getItem(ROWS.draft + at) ?? "",
    setDraft(at, text) {
      if (text === "") rows.removeItem(ROWS.draft + at);
      else rows.setItem(ROWS.draft + at, text);
    },
    draftUnkept: (at) => derived(rows.unkept, (names) => names.has(ROWS.draft + at)),
    editor: () => ({
      editor: readOne(EDITORS, rows.getItem(ROWS.editor), "none"),
      folder: rows.getItem(ROWS.cityFolder) ?? "",
    }),
    setEditor(next) {
      rows.setItem(ROWS.editor, next.editor);
      rows.setItem(ROWS.cityFolder, next.folder);
    },
    workbench,
    setWorkbench(next) {
      rows.setItem(ROWS.workbench, spelledWorkbench(next));
      workbench.set(next);
    },
  };
}

// The preferences this page runs on, one per document like the keymap:
// the shell, the settings screens and the face the page is drawn in all
// look at one record, and a second copy would let a change go unseen.
let shared: PreferenceDoor | undefined;

export function preferences(): PreferenceDoor {
  shared ??= loadPreferences(
    browserRows(),
    typeof navigator === "undefined" ? "" : navigator.language,
  );
  return shared;
}
