// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the box a person writes in knows: where a message lands and how
// that is spelled, what its three choosers offer and what a pick means,
// which verb a typed `/` line runs, and where unsent words are kept.
// `composer.svelte` draws these; this file decides them, so a chooser
// row and a menu row cannot come to mean two different things.

import { Option, Schema } from "effect";

import type { Belief, RunBelief } from "../../core/belief";
import { heldIn } from "../../core/belief/rooms";
import { EFFORTS, selectModel } from "../../core/commands";
import type { PreferenceDoor } from "../../core/prefs";
import type { Key, Lang } from "../../core/lang";
import { fill, say } from "../../core/lang";
import { MAYOR } from "../../core/route";
import { UNSTATED, offered, parse } from "../../core/slash";
import { reached } from "../../core/slash_hands";
import type { SessionHands, Slash, SlashHands } from "../../core/slash_hands";
import type { Sending } from "../../core/doing";
import { Address } from "../../wire";
import type { Command, Effort, RunPolicy, Seq } from "../../wire";
import type { View } from "../../core/route";
import type { PopoverColumn } from "../parts/popover";

// What the send control is spelled, for each of the three places a
// message can land. `queued` is the one a streaming page would
// otherwise hide: the run is inside a tool call, and the words wait for
// a boundary (client/Spec.lean §4-13).
export const SPELLING: Record<Sending, Key> = {
  dispatch: "talk_send",
  steer: "talk_steer",
  queued: "talk_steer_queued",
};

// How long a keystroke waits before the draft is written.
const DRAFT_MS = 500;

// ------------------------------------------------------------- the draft

// What was typed and not sent, for one place a person writes. The words
// not yet written and the timer that will write them travel with the
// door they go through, so the box cannot end up with one of the three
// and not the others.
//
// A keystroke schedules a write instead of making one: the draft door
// touches browser storage, and a write per key is a write per key. A
// change that empties or replaces the box writes at once - those are
// rare, and a reload right after a send must not resurrect the words
// that just went out.
export interface Draft {
  // Read once, when the box mounts: the door is a plain function, not a
  // signal.
  readonly read: string;
  // Words typed, to be written when the person pauses.
  keep(words: string): void;
  // Words placed in the box - sent away, transcribed, completed - to be
  // written now.
  replace(words: string): void;
  flush(): void;
}

export function draftAt(door: PreferenceDoor, at: string | undefined): Draft {
  let pending: string | null = null;
  let flushing: ReturnType<typeof setTimeout> | undefined = undefined;

  function flush(): void {
    if (flushing !== undefined) {
      clearTimeout(flushing);
      flushing = undefined;
    }
    if (pending === null || at === undefined) return;
    door.setDraft(at, pending);
    pending = null;
  }

  return {
    read: at === undefined ? "" : door.draft(at),
    keep(words) {
      pending = words;
      if (flushing !== undefined) clearTimeout(flushing);
      flushing = setTimeout(flush, DRAFT_MS);
    },
    replace(words) {
      pending = words;
      flush();
    },
    flush,
  };
}

// -------------------------------------------------------- the three pills

// One model the city could point the `main` face at: the endpoint that
// serves it and its id.
export interface Names {
  readonly endpoint: string;
  readonly model: string;
}

// That, with the name a person reads beside it.
export interface Served extends Names {
  readonly label: string;
}

// A model's two names travel as one row id, joined by a character no
// endpoint or model id may hold. The join and the split live here
// together, so the chooser and the command cannot disagree about where
// the seam is.
const MID = "\u0000";

export function modelValue(chosen: Names | undefined): string | null {
  return chosen === undefined ? null : `${chosen.endpoint}${MID}${chosen.model}`;
}

export function splitModel(value: string): Names | null {
  const [endpoint, model] = value.split(MID);
  if (endpoint === undefined || model === undefined || model === "") return null;
  return { endpoint, model };
}

// The model the session open in a room answers with: the model of the
// newest run since the session began that has called one, read from the
// room's runs oldest start first (`heldIn`). A session keeps the model
// it was opened with, so this, and not the city's next pick, is what a
// message sent here reaches, even while its newest run has not called
// yet; `null` when the session has made no call at all.
export function sessionModel(held: readonly RunBelief[], began: Seq | null): string | null {
  return held.filter((run) => (began === null || run.lastSeq > began) && run.model !== null).at(-1)?.model ?? null;
}

// A model pick changes the next main selection and never opens a session.
export type ModelMove =
  | { readonly kind: "select"; readonly names: Names }
  | { readonly kind: "stay" };

export function modelMove(value: string, session: string | null): ModelMove {
  const names = splitModel(value);
  return names === null || session !== null ? { kind: "stay" } : { kind: "select", names };
}

// The rows and the value of the model pill. While a session model is
// known the pill shows it, marked as kept by the session, and a model
// the city no longer serves still gets its row, so the pill never reads
// as naming no model while one answers.
function modelRows(lang: Lang, around: Around): { choices: Choice[]; value: string | null } {
  const session = around.session;
  const kept = say(lang, "talk_model_kept");
  const served = around.served.find((each) => each.model === session);
  const unserved = session === null || served !== undefined ? [] : [{ endpoint: "", model: session, label: kept }];
  const rows = [...unserved, ...around.served];
  return {
    choices: rows.map((each) => ({
      value: `${each.endpoint}${MID}${each.model}`,
      label: each.model === session ? `${each.model} · ${kept}` : each.model,
      note: each.label,
    })),
    value: modelValue(session === null ? around.chosen : (served ?? unserved.at(0))),
  };
}

// The level in force, spelled as the row that offers it: nobody having
// chosen is a row of its own (`core/slash.ts`'s `UNSTATED`), so the pill
// and the `/effort` line name the states the same way (client/Spec.lean
// §4-28). An id that is no level this build offers reads as unstated
// rather than as a level the frame would refuse.
export function effortLevel(value: string): Effort | null {
  if (value === UNSTATED) return null;
  return EFFORTS.find((known) => known === value) ?? null;
}

// What sending to a room means, in one line: the Mayor's own room, a
// bare building (a dispatch there opens a room of its own, client
// D6), or a room that carries on a conversation.
function roomNote(lang: Lang, room: string): string {
  if (room === MAYOR) return say(lang, "talk_room_note_mayor");
  const slash = room.indexOf("/");
  return slash < 0
    ? say(lang, "talk_room_note_building")
    : fill(say(lang, "talk_room_note_room"), { building: room.slice(0, slash) });
}

// The room a pill names, in the address spelling the wire brands.
export function decodeRoom(value: string): Address | null {
  return Option.getOrNull(Schema.decodeOption(Address)(value));
}

// Every room a person could move a conversation to: the buildings the
// city knows, and the rooms runs have already opened (the keys of
// belief's rooms index).
export function roomsKnown(buildings: Iterable<{ readonly addr: string }>, opened: Iterable<string>): string[] {
  const named = new Set<string>([MAYOR, ...opened]);
  for (const building of buildings) named.add(building.addr);
  return [...named].sort((a, b) => a.localeCompare(b));
}

// One row of a pill's menu: the value a pick sends, the short name a
// person reads, and one line saying what choosing it changes.
export interface Choice {
  readonly value: string;
  readonly label: string;
  readonly note?: string;
}

// One pill of the row under the box. `label` is the short name the
// trigger and a screen reader say; `about`, when there is more to say,
// heads the menu instead.
export interface Pill {
  readonly label: string;
  readonly about?: string;
  readonly placeholder: string;
  readonly choices: readonly Choice[];
  readonly value: string | null;
  readonly pick: (value: string) => void;
}

// What a pick does for each pill, named by the pill rather than passed
// in a row order only the builder could read.
export interface Picks {
  readonly model: (value: string) => void;
  readonly workspace: (value: string) => void;
  readonly effort: (value: string) => void;
}

// The page's hands a pick reaches for.
export interface PickHands {
  readonly send: (command: Command) => boolean;
  readonly go: (view: View) => void;
  readonly chooseEffort: (effort: Effort | null) => void;
}

// Picks configure the next dispatch; session creation belongs to slash commands.
export function picksFor(hands: PickHands, session: string | null): Picks {
  return {
    model: (value) => {
      const move = modelMove(value, session);
      if (move.kind === "stay") return;
      hands.send(selectModel(move.names.endpoint, move.names.model, "main"));
    },
    workspace: (value) => {
      const address = decodeRoom(value);
      if (address !== null) hands.go({ kind: "talk", address });
    },
    effort: (value) => {
      hands.chooseEffort(effortLevel(value));
    },
  };
}

// Everything the pills read off the page and the city, gathered so the
// three rows are built in one place.
export interface Around {
  readonly served: readonly Served[];
  readonly chosen: Names | undefined;
  // The model the session open here answers with (`sessionModel`).
  readonly session: string | null;
  readonly rooms: readonly string[];
  readonly here: Address | null;
  readonly effort: Effort | null;
}

// The three pill values - model, workspace, effort - shared by the entries
// in the row. The effort rows carry what a level costs beside the level
// itself, because this is where a person decides (client/Spec.lean §4-28).
export function pills(lang: Lang, around: Around, picks: Picks): readonly [Pill, Pill, Pill] {
  const levels: readonly (typeof UNSTATED | Effort)[] = [UNSTATED, ...EFFORTS];
  return [
    {
      label: say(lang, "talk_column_model"),
      placeholder: say(lang, "talk_no_model"),
      ...modelRows(lang, around),
      pick: picks.model,
    },
    {
      label: say(lang, "talk_column_workspace"),
      placeholder: say(lang, "talk_column_workspace"),
      choices: around.rooms.map((room) => ({ value: room, label: room, note: roomNote(lang, room) })),
      value: around.here,
      pick: picks.workspace,
    },
    {
      label: say(lang, "talk_column_effort"),
      placeholder: say(lang, "effort_unstated"),
      choices: levels.map((level) => ({
        value: level,
        label: say(lang, `effort_${level}`),
        note: say(lang, `effort_note_${level}`),
      })),
      value: around.effort ?? UNSTATED,
      pick: picks.effort,
    },
  ];
}

// ---------------------------------------------------------- the `/` menu

const COMMANDS = "commands";

// The menu's one column, built from the same table the Ctrl-K palette
// reads: a spelling, and beside it the shape its arguments take. A line
// no verb matches - a message that begins with a path - gets no menu at
// all, so the box's own Enter sends it rather than an empty list taking
// the key.
export function menuColumns(lang: Lang, line: string): readonly PopoverColumn[] {
  const verbs = offered(line);
  if (verbs.length === 0) return [];
  return [
    {
      id: COMMANDS,
      label: "talk_commands",
      rows: verbs.map((each) => ({
        id: each.spelling,
        label: each.spelling,
        secondary:
          each.grammar === ""
            ? say(lang, each.about)
            : `${each.grammar} · ${say(lang, each.about)}`,
      })),
    },
  ];
}

// A verb the menu offered, once. A verb that still needs its argument
// is answered with the line to complete into the box rather than run on
// nothing; a verb that ran answers `null`.
export function pickSlash(chosen: Slash, line: string, hands: SlashHands): string | null {
  const call = parse(line);
  const needs = chosen.grammar.startsWith("<");
  const typed = call !== null && call.verb === chosen.spelling;
  if (typed && (!needs || call.rest !== "")) {
    chosen.run(hands, call);
    return null;
  }
  return `${chosen.spelling} `;
}

// -------------------------------------------------------- the run in reach

// Everything a typed verb may reach for (`core/slash.ts` fills the same
// shape from the palette), minus the one conversion this file owns: a
// run belief becomes the run and position a verb acts on.
export interface Reach extends SessionHands {
  readonly command: (command: Command) => boolean;
  readonly go: (view: View) => void;
  readonly here: Address | null;
  readonly live: RunBelief | undefined;
  readonly belief: Belief;
  readonly models: readonly Served[];
  readonly effort: Effort | null;
  readonly setEffort: (effort: Effort | null) => void;
  readonly policy: RunPolicy;
  readonly setPolicy: (policy: RunPolicy) => void;
  readonly goal: string;
  readonly write: (line: string) => void;
}

export function slashHands(reach: Reach): SlashHands {
  return {
    ...reach,
    live: reached(reach.live),
    newest: (room) => reached(heldIn(reach.belief, room).at(-1)),
    models: reach.models.map((each) => ({ endpoint: each.endpoint, model: each.model })),
  };
}
