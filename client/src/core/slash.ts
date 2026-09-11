// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The verbs a person can type. One table, read by both places that
// offer them - the `/` menu above the composer and the Ctrl-K palette -
// so a button spelled `/stop` and a line somebody types cannot drift
// into meaning two different things.
//
// A command holds four things and no more: how it is spelled, what it
// takes after the spelling, the phrase that explains it, and what it
// does. What it does is written against `SlashHands`, which is every
// capability a verb may reach for; a view fills the hands in and stays
// out of the verbs. That is why this file names no view, no signal and
// no query: the same table runs from a text box and from a palette.

import { Option } from "effect";

import { Address } from "./address";
import {
  TEMPLATES,
  cancel,
  createBuilding,
  dispatch,
  fork,
  halt,
  release,
  selectModel,
  steer,
} from "./commands";
import type { Template } from "./commands";
import type { Key } from "./lang";
import { EFFORTS } from "./prefs";
import { fromFragment } from "./route";
import type { View } from "./route";
import type { Command, Effort, RunId, Seq } from "../wire";

// A run a verb can act on: which one, and how far it has got. `/fork`
// needs both, `/steer` and `/stop` only the first.
export interface Reached {
  readonly run: RunId;
  readonly at: Seq;
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
  // How hard a run this verb opens should think. It comes from the
  // selector beside the box rather than from the line, because that is
  // where the person set it.
  readonly effort: Effort;
  readonly setEffort: (effort: Effort) => void;
  // What a new run is told to aim at, already in the person's language.
  readonly goal: string;
  // The line in the box: emptied by `/clear`, refilled by `/help`.
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

export interface Slash {
  readonly spelling: string;
  // What may follow the spelling, in the notation the help line shows:
  // `<required>`, `[optional]`, `a|b` for a choice.
  readonly grammar: string;
  readonly about: Key;
  readonly run: (hands: SlashHands, call: SlashCall) => void;
}

// The pages `/go` offers, as the fragment heads `route` already reads.
// The names are the table; the meaning of each name stays where every
// other spelling of a route is resolved.
export const PAGES: readonly string[] = [
  "talk",
  "city",
  "setup",
  "mcp",
  "record",
  "cost",
  "welcome",
  "gallery",
];

const ALL = "--all";
const QUEUED = "--queued";

function addressed(raw: string | undefined): Address | null {
  if (raw === undefined || raw === "" || raw.startsWith("-")) {
    return null;
  }
  return Option.getOrNull(Address.option(raw));
}

function paged(name: string | undefined): View | null {
  if (name === undefined || !PAGES.includes(name)) {
    return null;
  }
  return Option.getOrNull(fromFragment(`#/${name}`));
}

// `/stop` and `/release` take the same three shapes, and the pair would
// otherwise be one body copied twice with a verb swapped.
function scoped(hands: SlashHands, call: SlashCall, halting: boolean): void {
  if (call.words.includes(ALL)) {
    hands.command(halting ? halt("city") : release("city"));
    return;
  }
  const named = addressed(call.words.at(0));
  if (named !== null) {
    hands.command(halting ? halt({ building: named }) : release({ building: named }));
    return;
  }
  // Bare `/stop` is the button beside the box: end the run in front of
  // the person. Bare `/release` has no such run to name, so it reaches
  // the only scope a person who typed nothing can have meant.
  if (halting) {
    if (hands.live !== null) {
      hands.command(cancel(hands.live.run));
    }
    return;
  }
  hands.command(release("city"));
}

export const SLASH: readonly Slash[] = [
  {
    spelling: "/dispatch",
    grammar: "<task>",
    about: "slash_dispatch",
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
    grammar: "[--queued] <text>",
    about: "slash_steer",
    run: (hands, call) => {
      const said = call.words.filter((word) => word !== QUEUED).join(" ");
      if (hands.live === null || said === "") return;
      hands.command(steer(hands.live.run, said));
      hands.write("");
    },
  },
  {
    spelling: "/stop",
    grammar: "[addr|--all]",
    about: "slash_stop",
    run: (hands, call) => {
      scoped(hands, call, true);
      hands.write("");
    },
  },
  {
    spelling: "/release",
    grammar: "[addr|--all]",
    about: "slash_release",
    run: (hands, call) => {
      scoped(hands, call, false);
      hands.write("");
    },
  },
  {
    spelling: "/raise",
    grammar: "<addr> [minimal|confidential|hall]",
    about: "slash_raise",
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
    spelling: "/fork",
    grammar: "<addr>",
    about: "slash_fork",
    run: (hands, call) => {
      const room = call.words.at(0);
      if (room === undefined) return;
      const newest = hands.newest(room);
      if (newest === null) return;
      hands.command(fork(newest.run, newest.at, null));
      hands.write("");
    },
  },
  {
    spelling: "/model",
    grammar: "<id>",
    about: "slash_model",
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
    grammar: EFFORTS.join("|"),
    about: "slash_effort",
    run: (hands, call) => {
      const asked = call.words.at(0);
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
    run: (hands) => {
      hands.go({ kind: "mcp" });
      hands.write("");
    },
  },
  {
    spelling: "/doctor",
    grammar: "",
    about: "slash_doctor",
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
    run: (hands) => {
      // A lone slash is what opens the whole list, so `/help` is that
      // list rather than a second page describing it.
      hands.write("/");
    },
  },
  {
    spelling: "/clear",
    grammar: "",
    about: "slash_clear",
    run: (hands) => {
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

function shared(spellings: readonly string[]): string {
  const first = spellings.at(0);
  if (first === undefined) {
    return "";
  }
  let out = first;
  for (const spelling of spellings) {
    while (!spelling.startsWith(out)) {
      out = out.slice(0, -1);
    }
  }
  return out;
}

// What Tab makes of a line: the one match completed, or the longest
// prefix every match shares. A line Tab cannot improve comes back
// unchanged, so the caller has nothing to decide.
export function completed(line: string): string {
  const call = parse(line);
  if (call === null) {
    return line;
  }
  const hits = SLASH.filter((known) => known.spelling.startsWith(call.verb));
  const only = hits.length === 1 ? hits.at(0) : undefined;
  if (only !== undefined) {
    return call.rest === "" ? `${only.spelling} ` : line;
  }
  const prefix = shared(hits.map((known) => known.spelling));
  return prefix.length > call.verb.length ? prefix : line;
}
