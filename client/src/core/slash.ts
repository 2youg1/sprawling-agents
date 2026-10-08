// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The verbs a person can type. One table, read by both places that
// offer them - the `/` menu above the composer and the Accel-/ palette -
// so a button spelled `/stop` and a line somebody types cannot drift
// into meaning two different things.
//
// A command holds five things and no more: how it is spelled, what it
// takes after the spelling, the phrase that explains it, the palette
// section it is listed under, and what it does. What it does is
// written against `SlashHands` (`slash_hands.ts`), which is every
// capability a verb may reach for; a view fills the hands in and stays
// out of the verbs. That is why this file names no view, no signal and
// no query: the same table runs from a text box and from a palette.

import { Option, Schema } from "effect";

import { Address } from "../wire";
import type { AdmissionRequirement } from "../wire";
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
import { askQuit } from "./quitting";
import { PAGES, page } from "./route";
import { given, stripped } from "./tags";
import type { View } from "./route";
import { CITY } from "./scope";
import type { Slash, SlashCall, SlashHands } from "./slash_hands";
import { compact, fresh, retag } from "./slash_session";

const ALL = "--all";

// The two spellings a button elsewhere shows as its own name: the verb
// that cancels the run in front of the person, and the one that lets
// the whole city work again. Written here once, so the button and the
// line a person types are one spelling.
export const STOP = "/stop";
const RELEASE = "/release";
export const RELEASE_ALL = `${RELEASE} ${ALL}`;

// The short words `/admit` takes, each naming the admission requirement
// the next run's work must meet; the bare verb goes back to the
// building's own checks.
const ADMITS: Readonly<Record<string, AdmissionRequirement>> = {
  standing: "standing",
  tested: "tested",
  contract: "contract_kept",
  double: "double_validated",
};

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

export const SLASH: readonly Slash[] = [
  {
    spelling: "/dispatch",
    grammar: "<task>",
    about: "slash_dispatch",
    section: "sessions",
    run: (hands, call) => {
      if (hands.here === null || call.rest === "") return;
      hands.command(
        dispatch({
          addr: hands.here,
          task: call.rest,
          goal: hands.goal,
          effort: hands.effort,
          policy: hands.policy,
        }),
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
    spelling: STOP,
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
    spelling: RELEASE,
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
    spelling: "/compact",
    grammar: "",
    about: "slash_compact",
    section: "sessions",
    run: compact,
  },
  {
    spelling: "/tag",
    grammar: "<name>",
    about: "slash_tag",
    section: "sessions",
    run: (hands, call) => {
      retag(hands, call, given);
    },
  },
  {
    spelling: "/untag",
    grammar: "<name>",
    about: "slash_untag",
    section: "sessions",
    run: (hands, call) => {
      retag(hands, call, stripped);
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
    spelling: "/admit",
    grammar: `[${Object.keys(ADMITS).join("|")}]`,
    about: "slash_admit",
    section: "actions",
    run: (hands, call) => {
      const admit = ADMITS[call.words.at(0) ?? "standing"];
      if (admit === undefined) return;
      hands.setPolicy({ ...hands.policy, admit });
      hands.write("");
    },
  },
  {
    spelling: "/room",
    grammar: "<addr>",
    about: "slash_room",
    section: "navigation",
    run: (hands, call) => {
      const address = addressed(call.words.at(0));
      if (address === null) return;
      hands.go({ kind: "talk", address });
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
    // The ACP agents page; a launch spec typed after the verb is pasted
    // into the page's box by hand, because the box is where the city's
    // reading of it is shown before anything is added.
    spelling: "/acp",
    grammar: "",
    about: "slash_acp",
    section: "navigation",
    run: (hands) => {
      hands.go({ kind: "setup", group: "agents" });
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
    // Closing the city is a question first: with runs going, whether to
    // wait for them (`views/quit.svelte`).
    spelling: "/quit",
    grammar: "",
    about: "slash_quit",
    section: "actions",
    run: (hands) => {
      askQuit();
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
      hands.go({ kind: "run", run: shown.run, lens: "changes" });
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
// is its grammar, not every other verb.
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
