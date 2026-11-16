// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

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
import { PAGE_VERBS } from "./slash_pages";
import { given, stripped } from "./tags";
import { CITY } from "./scope";
import { readWorkspace, workspaceDestination } from "./route";
import type { Slash, SlashCall, SlashHands } from "./slash_hands";
import { compact, fresh, retag } from "./slash_session";

const ALL = "--all";

export const STOP = "/stop";
const RELEASE = "/release";
export const RELEASE_ALL = `${RELEASE} ${ALL}`;

const ADMITS: Readonly<Record<string, AdmissionRequirement>> = {
  standing: "standing",
  tested: "tested",
  contract: "contract_kept",
  double: "double_validated",
};

export const UNSTATED = "unstated" as const;

function addressed(raw: string | undefined): Address | null {
  if (raw === undefined || raw === "" || raw.startsWith("-")) {
    return null;
  }
  return Option.getOrNull(Schema.decodeOption(Address)(raw));
}

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
    grammar: "[addr] [minimal|confidential|hall]",
    about: "slash_raise",
    section: "actions",
    run: (hands, call) => {
      if (call.words.length === 0) {
        hands.go({ kind: "raise" });
        hands.write("");
        return;
      }
      const at = addressed(call.words.at(0));
      const asked = call.words.at(1) ?? "minimal";
      const template = TEMPLATES.find((known): known is Template => known === asked);
      if (at === null || template === undefined) return;
      hands.command(createBuilding(at, template, null));
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
        askFork();
        hands.write("");
        return;
      }
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
      if (call.rest.startsWith("-")) return;
      const view = readWorkspace(call.rest);
      if (view === null) return;
      hands.go(view);
      hands.write("");
    },
  },
  {
    spelling: "/cd",
    grammar: "[addr]",
    about: "slash_cd",
    section: "navigation",
    run: (hands, call) => {
      if (call.words.length === 0) {
        hands.go({ kind: "city" });
        hands.write("");
        return;
      }
      if (call.rest.startsWith("-") || hands.buildings === undefined) return;
      const view = readWorkspace(call.rest);
      if (view === null) return;
      const destination = workspaceDestination(view, hands.buildings);
      if (destination === null) return;
      hands.go(destination);
      hands.write("");
    },
  },
  {
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
      hands.write("/");
    },
  },
  ...PAGE_VERBS,
];

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
