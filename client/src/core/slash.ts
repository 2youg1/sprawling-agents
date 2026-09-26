// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The verbs a person can type. One table, read by both places that
// offer them - the `/` menu above the composer and the Ctrl-K palette -
// so a button spelled `/stop` and a line somebody types cannot drift
// into meaning two different things.
//
// A command holds five things and no more: how it is spelled, what it
// takes after the spelling, the phrase that explains it, the palette
// section it is listed under, and what it does. What it does is written against `SlashHands`, which is every
// capability a verb may reach for; a view fills the hands in and stays
// out of the verbs. That is why this file names no view, no signal and
// no query: the same table runs from a text box and from a palette.

import { Option, Schema } from "effect";

import { Address } from "../wire";
import type { RunBelief } from "./belief";
import {
  EFFORTS,
  TEMPLATES,
  cancel,
  createBuilding,
  dispatch,
  halt,
  openSession,
  release,
  selectModel,
  steer,
} from "./commands";
import type { Template } from "./commands";
import { askFork } from "./forking";
import type { Key } from "./lang";
import { PAGES, page } from "./route";
import type { View } from "./route";
import { CITY } from "./scope";
import type { Command, Effort, RunId, Seq } from "../wire";

// A run a verb can act on: which one, and how far it has got. `/steer`,
// `/stop` and `/diff` need the first, and a branch needs both.
export interface Reached {
  readonly run: RunId;
  readonly at: Seq;
}

// The run a view has in hand, as a verb reaches it, and nothing when
// the view has none. It lives beside the type rather than beside each
// caller: the composer and the palette both fill `SlashHands`, and
// each kept a copy of this one conversion.
export function reached(run: RunBelief | undefined): Reached | null {
  return run === undefined ? null : { run: run.run, at: run.lastSeq };
}

// One model the city could be pointed at, named the way the settings
// page names it: the endpoint that serves it, and its id.
export interface Offered {
  readonly endpoint: string;
  readonly model: string;
}

// Everything a verb may reach for, and nothing a view could not hand
// over from what it already has. A field that is null is a capability
// this place does not have - there is no room to dispatch into on the
// run page - and a verb that needs it does nothing rather than guess.
export interface SlashHands {
  readonly command: (command: Command) => boolean;
  readonly go: (view: View) => void;
  // The room the box speaks to, and the run going in it.
  readonly here: Address | null;
  readonly live: Reached | null;
  // The newest run in a room somebody named by hand.
  readonly newest: (room: string) => Reached | null;
  readonly models: readonly Offered[];
  // How hard a run this verb opens should think, or `null` when the
  // person has chosen no level and the provider decides. It comes from
  // the selector beside the box rather than from the line, because
  // that is where the person set it.
  readonly effort: Effort | null;
  readonly setEffort: (effort: Effort | null) => void;
  // What a new run is told to aim at, already in the person's language.
  readonly goal: string;
  // The line in the box: emptied by a verb that ran, refilled by `/help`.
  readonly write: (line: string) => void;
}

// A typed line, cut where the first space is: the verb as spelled, the
// arguments split, and the arguments as typed. `/steer` needs the
// unsplit form, because what follows it is a sentence rather than a
// list of words.
export interface SlashCall {
  readonly verb: string;
  readonly words: readonly string[];
  readonly rest: string;
}

// The palette's groups, in its order. A verb names its own, so a verb
// added or renamed cannot fall into a group nobody chose for it.
export const SECTIONS = ["actions", "navigation", "sessions"] as const;
export type Section = (typeof SECTIONS)[number];

export interface Slash {
  readonly spelling: string;
  // What may follow the spelling, in the notation the help line shows:
  // `<required>`, `[optional]`, `a|b` for a choice.
  readonly grammar: string;
  readonly about: Key;
  readonly section: Section;
  readonly run: (hands: SlashHands, call: SlashCall) => void;
}

const ALL = "--all";
const CARRY = "--carry";

// The word for an effort nobody chose. The page keeps that state as
// `null` and the frame leaves the field out; this is its one written
// form - what a person types after `/effort`, and the id the selector
// over the composer gives the row that reaches it, so the line and
// the menu cannot come to mean two different things.
//
// The assertion keeps the word one word: without it, a menu that
// builds its rows from `[UNSTATED, ...EFFORTS]` widens the whole list
// to `string` and loses the phrase key each row is named by.
export const UNSTATED = "unstated" as const;

function addressed(raw: string | undefined): Address | null {
  if (raw === undefined || raw === "" || raw.startsWith("-")) {
    return null;
  }
  return Option.getOrNull(Schema.decodeOption(Address)(raw));
}

// Only what `/go` advertises, resolved by the router: a page this
// build cannot name is not a page this verb opens.
function paged(name: string | undefined): View | null {
  if (name === undefined || !PAGES.includes(name)) {
    return null;
  }
  return Option.getOrNull(page(name));
}

// `/halt` and `/release` take the same three shapes, and the pair would
// otherwise be one body copied twice with a frame swapped. A bare verb
// or `--all` reaches the city; a word that is not an address sends
// nothing and leaves the line to edit, because widening a typo to the
// whole city is the one reading the person cannot have meant.
function scoped(hands: SlashHands, call: SlashCall, frame: typeof halt): void {
  const first = call.words.at(0);
  if (first === undefined || call.words.includes(ALL)) {
    hands.command(frame(CITY));
    hands.write("");
    return;
  }
  const named = addressed(first);
  if (named === null) return;
  hands.command(frame({ building: named }));
  hands.write("");
}

// `/new` and `/clear` are one verb: a new session here, with nothing
// from the last one unless `--carry` says so. A room with no handoff
// answers `carried: false` rather than refusing, so the word passes on.
function fresh(hands: SlashHands, call: SlashCall): void {
  if (hands.here === null) return;
  const carry = call.words.includes(CARRY) ? "handoff" : "nothing";
  hands.command(openSession(hands.here, carry, null));
  hands.write("");
}

export const SLASH: readonly Slash[] = [
  {
    spelling: "/dispatch",
    grammar: "<task>",
    about: "slash_dispatch",
    section: "sessions",
    run: (hands, call) => {
      if (hands.here === null || call.rest === "") return;
      hands.command(
        dispatch({ addr: hands.here, task: call.rest, goal: hands.goal, effort: hands.effort }),
      );
      hands.write("");
    },
  },
  {
    spelling: "/steer",
    grammar: "<text>",
    about: "slash_steer",
    section: "sessions",
    run: (hands, call) => {
      if (hands.live === null || call.rest === "") return;
      hands.command(steer(hands.live.run, call.rest));
      hands.write("");
    },
  },
  {
    // Cancel, the run in front of the person; the wider brake is `/halt`.
    spelling: "/stop",
    grammar: "",
    about: "slash_stop",
    section: "sessions",
    run: (hands) => {
      if (hands.live === null) return;
      hands.command(cancel(hands.live.run));
      hands.write("");
    },
  },
  {
    spelling: "/halt",
    grammar: "[addr|--all]",
    about: "slash_halt",
    section: "actions",
    run: (hands, call) => {
      scoped(hands, call, halt);
    },
  },
  {
    spelling: "/release",
    grammar: "[addr|--all]",
    about: "slash_release",
    section: "actions",
    run: (hands, call) => {
      scoped(hands, call, release);
    },
  },
  {
    spelling: "/raise",
    grammar: "<addr> [minimal|confidential|hall]",
    about: "slash_raise",
    section: "actions",
    run: (hands, call) => {
      const at = addressed(call.words.at(0));
      const asked = call.words.at(1) ?? "minimal";
      const template = TEMPLATES.find((known): known is Template => known === asked);
      if (at === null || template === undefined) return;
      hands.command(createBuilding(at, template));
      hands.write("");
    },
  },
  {
    spelling: "/new",
    grammar: "[--carry]",
    about: "slash_new",
    section: "sessions",
    run: fresh,
  },
  {
    spelling: "/clear",
    grammar: "",
    about: "slash_clear",
    section: "sessions",
    run: (hands) => {
      fresh(hands, { verb: "/clear", words: [], rest: "" });
    },
  },
  {
    spelling: "/fork",
    grammar: "[addr]",
    about: "slash_fork",
    section: "sessions",
    run: (hands, call) => {
      const room = addressed(call.words.at(0));
      if (room === null) {
        // No address means the point is in this conversation: the
        // picker above the box knows the turns and where each one sits.
        askFork();
        hands.write("");
        return;
      }
      // With an address the branch goes to that room, from the tail of
      // the newest run there: forking into another room predates the
      // picker and keeps its meaning.
      const newest = hands.newest(room);
      if (newest === null) return;
      hands.command(openSession(room, "nothing", { run: newest.run, at_seq: newest.at }));
      hands.write("");
    },
  },
  {
    spelling: "/model",
    grammar: "<id>",
    about: "slash_model",
    section: "actions",
    run: (hands, call) => {
      const id = call.words.at(0);
      const found = hands.models.find((offered) => offered.model === id);
      if (found === undefined) return;
      hands.command(selectModel(found.endpoint, found.model, "main"));
      hands.write("");
    },
  },
  {
    spelling: "/effort",
    grammar: `[${[UNSTATED, ...EFFORTS].join("|")}]`,
    about: "slash_effort",
    section: "actions",
    run: (hands, call) => {
      const asked = call.words.at(0);
      // `/effort unstated`, and bare `/effort`, both take the choice
      // back off the table and leave it to the provider, which is what
      // an unset selector means and what `"none"` - think as little as
      // possible - does not. The word is named in the grammar because
      // a person who has chosen a level needs to see the way back; the
      // bare form stays because omitting the argument is what the
      // request itself does. A level nobody offers changes nothing.
      if (asked === undefined || asked === UNSTATED) {
        hands.setEffort(null);
        hands.write("");
        return;
      }
      const level = EFFORTS.find((known) => known === asked);
      if (level === undefined) return;
      hands.setEffort(level);
      hands.write("");
    },
  },
  {
    spelling: "/go",
    grammar: PAGES.join("|"),
    about: "slash_go",
    section: "navigation",
    run: (hands, call) => {
      const to = paged(call.words.at(0));
      if (to === null) return;
      hands.go(to);
      hands.write("");
    },
  },
  {
    spelling: "/mcp",
    grammar: "",
    about: "slash_mcp",
    section: "navigation",
    run: (hands) => {
      hands.go({ kind: "mcp" });
      hands.write("");
    },
  },
  {
    spelling: "/doctor",
    grammar: "",
    about: "slash_doctor",
    section: "navigation",
    run: (hands) => {
      // The machine report is the welcome's first step, and that page
      // asks the `doctor` query again when it opens.
      hands.go({ kind: "welcome" });
      hands.write("");
    },
  },
  {
    spelling: "/help",
    grammar: "",
    about: "slash_help",
    section: "actions",
    run: (hands) => {
      // A lone slash is what opens the whole list, so `/help` is that
      // list rather than a second page describing it.
      hands.write("/");
    },
  },
  {
    // The changes are a lens of the run page: open this run's, or the newest here.
    spelling: "/diff",
    grammar: "",
    about: "slash_diff",
    section: "navigation",
    run: (hands) => {
      const shown = hands.live ?? (hands.here === null ? null : hands.newest(hands.here));
      if (shown === null) return;
      hands.go({ kind: "run", run: shown.run });
      hands.write("");
    },
  },
];

// A line that begins with a slash, cut into verb and arguments. `null`
// for anything else, which is the answer that keeps every caller from
// asking `startsWith` for itself.
export function parse(line: string): SlashCall | null {
  if (!line.startsWith("/")) {
    return null;
  }
  const space = line.search(/\s/);
  const verb = space < 0 ? line : line.slice(0, space);
  const rest = space < 0 ? "" : line.slice(space + 1).trim();
  return { verb, words: rest === "" ? [] : rest.split(/\s+/), rest };
}

export function find(verb: string): Slash | undefined {
  return SLASH.find((known) => known.spelling === verb);
}

// What the menu shows for a line. A verb already typed in full, with a
// space after it, narrows the list to itself: what a person needs then
// is its grammar, not thirteen other verbs.
export function offered(line: string): readonly Slash[] {
  const call = parse(line);
  if (call === null) {
    return [];
  }
  const whole = find(call.verb);
  if (whole !== undefined && /\s/.test(line)) {
    return [whole];
  }
  return SLASH.filter((known) => known.spelling.startsWith(call.verb));
}
