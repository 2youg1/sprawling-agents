// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the box a person writes in knows: where a message lands and how
// that is spelled, what its three choosers offer and what a pick means,
// which verb a typed `/` line runs, and what a key in the box does.
// `composer.svelte` seats these and `composer.look.svelte` draws them;
// this file decides them, so a chooser row and a menu row cannot come to
// mean two different things. Where unsent words are kept is `draft.ts`.

import { Option, Schema } from "effect";
import type { Snippet } from "svelte";
import type { HTMLFormAttributes, HTMLTextareaAttributes } from "svelte/elements";

import type { Belief, RunBelief } from "../../core/belief";
import { heldIn } from "../../core/belief/rooms";
import { selectModel } from "../../core/commands";
import type { Key, Lang } from "../../core/lang";
import { fill, say } from "../../core/lang";
import { MAYOR } from "../../core/route";
import { completed } from "../../core/completion";
import { offered, parse } from "../../core/slash";
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

/** The list size after which a chooser offers filtering. */
export const FILTER_AFTER = 8;

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

export function modelMove(names: Names, session: string | null): ModelMove {
  return session !== null ? { kind: "stay" } : { kind: "select", names };
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

// What a pick on the settings row does: the model picker's two choices,
// and the workspace chip's.
export interface Picks {
  readonly model: (names: Names) => void;
  readonly workspace: (value: string) => void;
  readonly effort: (level: Effort) => void;
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
    model: (names) => {
      const move = modelMove(names, session);
      if (move.kind === "stay") return;
      hands.send(selectModel(move.names.endpoint, move.names.model, "main"));
    },
    workspace: (value) => {
      const address = decodeRoom(value);
      if (address !== null) hands.go({ kind: "talk", address });
    },
    effort: (level) => {
      hands.chooseEffort(level);
    },
  };
}

// The rooms the workspace chip offers, and the one this box speaks to.
export interface Around {
  readonly rooms: readonly string[];
  readonly here: Address | null;
}

// The workspace chip: every room a conversation could move to, each
// with one line saying what sending there means.
export function workspacePill(lang: Lang, around: Around, pick: (value: string) => void): Pill {
  return {
    label: say(lang, "talk_column_workspace"),
    placeholder: say(lang, "talk_column_workspace"),
    choices: around.rooms.map((room) => ({ value: room, label: room, note: roomNote(lang, room) })),
    value: around.here,
    pick,
  };
}

// ---------------------------------------------------------- the `/` menu

const COMMANDS = "commands";

// What `/model` and `/effort` complete their argument from: the models
// the city serves, and the levels the model in force offers.
export interface Arguments {
  readonly models: readonly Served[];
  readonly levels: readonly Effort[];
}

// The two verbs whose argument the menu completes, each with its rows.
const COMPLETED = { model: "/model", effort: "/effort" } as const;

// The menu's one column, built from the same table the Ctrl-K palette
// reads: a spelling, and beside it the shape its arguments take. A line
// no verb matches - a message that begins with a path - gets no menu at
// all, so the box's own Enter sends it rather than an empty list taking
// the key. `/model` and `/effort` typed in full list their arguments
// instead, from what the city offers now; each row's id is the whole
// line it runs.
export function menuColumns(lang: Lang, line: string, offers: Arguments): readonly PopoverColumn[] {
  const verbs = offered(line);
  if (verbs.length === 0) return [];
  const argued = argumentRows(lang, line, offers);
  if (argued !== null) return [argued];
  return [
    {
      id: COMMANDS,
      label: "talk_commands",
      rows: verbs.map((each) => ({
        id: each.spelling,
        label: each.spelling,
        secondary: each.spelling === COMPLETED.effort ? levelsNote(lang, offers.levels) : grammarNote(lang, each),
      })),
    },
  ];
}

function grammarNote(lang: Lang, verb: Slash): string {
  return verb.grammar === "" ? say(lang, verb.about) : `${verb.grammar} · ${say(lang, verb.about)}`;
}

// `/effort` says which levels the model in force offers, rather than
// every level the city can spell.
function levelsNote(lang: Lang, levels: readonly Effort[]): string {
  return levels.length === 0 ? say(lang, "slash_effort_none_offered") : fill(say(lang, "slash_effort_offered"), { levels: levels.join(" · ") });
}

function argumentRows(lang: Lang, line: string, offers: Arguments): PopoverColumn | null {
  const call = parse(line);
  if (call === null || !/\s/.test(line) || call.words.length > 1) return null;
  const typed = (call.words.at(0) ?? "").toLowerCase();
  switch (call.verb) {
    case COMPLETED.model:
      return {
        id: COMMANDS,
        label: "talk_column_model",
        rows: offers.models
          .filter((each, at, all) => each.model.toLowerCase().includes(typed) && all.findIndex((other) => other.model === each.model) === at)
          .map((each) => ({ id: `${COMPLETED.model} ${each.model}`, label: each.model, secondary: each.label })),
      };
    case COMPLETED.effort:
      return offers.levels.length === 0
        ? null
        : {
            id: COMMANDS,
            label: "talk_column_effort",
            rows: offers.levels
              .filter((level) => level.startsWith(typed))
              .map((level) => ({ id: `${COMPLETED.effort} ${level}`, label: level, secondary: say(lang, `effort_note_${level}`) })),
          };
    default:
      return null;
  }
}

// What Tab makes of the line when the menu's cursor is on a row: an
// argument row is the whole line it runs, so Tab takes it as written;
// a verb row goes to the verb table's own completion.
export function completedAt(line: string, pointed: string | undefined): string {
  return pointed !== undefined && /\s/.test(pointed) ? `${pointed} ` : completed(line, pointed);
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

// ------------------------------------------------------ a key in the box

// What one key pressed in the box does. `complete` takes the menu's
// verb into the box, `menu` offers the key to the open `/` menu first,
// `recall` brings back what was last sent here, `send` hands the words
// on, and `type` leaves the key to the box. A key that confirms an input
// method's composition belongs to the input method, so it is always
// `type`: an Enter that picks a candidate must not send half a sentence.
export type BoxKey = "complete" | "menu" | "recall" | "send" | "type";

export interface KeyPressed {
  readonly key: string;
  readonly shiftKey: boolean;
  readonly isComposing: boolean;
}

export interface BoxState {
  // Whether the `/` menu is over the box with rows in it.
  readonly menu: boolean;
  readonly empty: boolean;
}

// A key the menu does not take is asked again with `menu: false`, so the
// box keeps its own keys under an open menu.
export function boxKey(pressed: KeyPressed, box: BoxState): BoxKey {
  if (pressed.isComposing) return "type";
  if (box.menu) return pressed.key === "Tab" ? "complete" : "menu";
  if (pressed.key === "ArrowUp" && box.empty) return "recall";
  if (pressed.key === "Enter" && !pressed.shiftKey) return "send";
  return "type";
}

// ---------------------------------------------------------- the look

// What `composer.look.svelte` is handed (docs/frontend-method.md §7I):
// the form's wiring - its name, the submit, and the three handlers of a
// drop target - and the text box's, each as one bag the look spreads on
// its element, whether a drag is over the box, and the parts the look
// places around the words, each already seated.
export interface ComposerLook {
  readonly form: HTMLFormAttributes;
  readonly over: boolean;
  readonly box: HTMLTextareaAttributes;
  // The `/` menu, while it is open over the box.
  readonly menu: Snippet | undefined;
  // What stands above the words in the band (the last thing said).
  readonly band: Snippet | undefined;
  // What stands at the end of the words' row: the microphone, the
  // context ring and the coin key.
  readonly keys: Snippet;
  // The line under the words.
  readonly line: Snippet;
  // Under the line: the send receipt, the draft the browser did not
  // keep, the files a drop did not keep, and the settings row.
  readonly under: Snippet;
}
