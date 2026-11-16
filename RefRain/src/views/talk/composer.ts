// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Snippet } from "svelte";
import type { HTMLFormAttributes, HTMLTextareaAttributes } from "svelte/elements";

import type { Belief, RunBelief } from "../../core/belief";
import { heldIn } from "../../core/belief/rooms";
import { configureCity, selectModel } from "../../core/commands";
import type { Key, Lang } from "../../core/lang";
import { fill, say } from "../../core/lang";
import { completed } from "../../core/completion";
import { offered, parse } from "../../core/slash";
import { reached } from "../../core/slash_hands";
import type { SessionHands, Slash, SlashHands } from "../../core/slash_hands";
import type { Sending } from "../../core/doing";
import type { Address } from "../../wire";
import type {
  Answer,
  BuildingProgress,
  Command,
  Effort,
  EndpointsAnswer,
  RunPolicy,
  Seq,
} from "../../wire";
import type { View } from "../../core/route";
import type { PopoverColumn } from "../parts/popover";
import { offersOf } from "./picker";
import type { PickerFacts } from "./picker_scene";

export const SPELLING: Record<Sending, Key> = {
  dispatch: "talk_send",
  steer: "talk_steer",
  queued: "talk_steer_queued",
};

export interface Names {
  readonly endpoint: string;
  readonly model: string;
}

export interface Served extends Names {
  readonly label: string;
}

export function sessionModel(held: readonly RunBelief[], began: Seq | null): string | null {
  return (
    held.filter((run) => (began === null || run.lastSeq > began) && run.model !== null).at(-1)
      ?.model ?? null
  );
}

export function startedIn(held: readonly RunBelief[], began: Seq | undefined): boolean {
  return held.some((run) => run.lastSeq > (began ?? 0));
}

export type ModelMove =
  { readonly kind: "select"; readonly names: Names } | { readonly kind: "stay" };

export function modelMove(names: Names, session: string | null): ModelMove {
  return session !== null ? { kind: "stay" } : { kind: "select", names };
}

export interface Picks {
  readonly model: (names: Names) => void;
  readonly effort: (level: Effort) => void;
}

export interface PickHands {
  readonly send: (command: Command) => boolean;
  readonly chooseEffort: (effort: Effort | null) => void;
}

export function picksFor(hands: PickHands, session: string | null): Picks {
  return {
    model: (names) => {
      const move = modelMove(names, session);
      if (move.kind === "stay") return;
      hands.send(selectModel(move.names.endpoint, move.names.model, "main"));
    },
    effort: (level) => {
      if (hands.send(configureCity(null, level))) hands.chooseEffort(null);
    },
  };
}

export interface Offered {
  readonly models: readonly Served[];
  readonly picker: PickerFacts;
  readonly argued: Arguments;
}

export function offeredBy(
  answer: EndpointsAnswer | undefined,
  stated: Effort | null,
  picks: Picks,
): Offered {
  const endpoints = answer?.endpoints ?? [];
  const main = answer?.chosen.find((each) => each.tag === "main");
  const models = endpoints.flatMap((endpoint) =>
    endpoint.models.map((row) => ({
      endpoint: endpoint.name,
      label: endpoint.label,
      model: row.id,
    })),
  );
  const inForce = offersOf(endpoints).find(
    (each) => each.endpoint === main?.endpoint && each.model === main.model,
  );
  return {
    models,
    picker: { endpoints, chosen: main, stated, pick: { model: picks.model, level: picks.effort } },
    argued: { models, levels: inForce?.facts.thinking.levels ?? [] },
  };
}

export function configLevel(answer: Answer | undefined): Effort | null {
  return answer !== undefined && "config" in answer ? (answer.config.effort?.effort ?? null) : null;
}

const COMMANDS = "commands";

export interface Arguments {
  readonly models: readonly Served[];
  readonly levels: readonly Effort[];
}

const COMPLETED = { model: "/model", effort: "/effort" } as const;

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
        secondary:
          each.spelling === COMPLETED.effort
            ? levelsNote(lang, offers.levels)
            : grammarNote(lang, each),
      })),
    },
  ];
}

function grammarNote(lang: Lang, verb: Slash): string {
  return verb.grammar === "" ? say(lang, verb.about) : `${verb.grammar} · ${say(lang, verb.about)}`;
}

function levelsNote(lang: Lang, levels: readonly Effort[]): string {
  return levels.length === 0
    ? say(lang, "slash_effort_none_offered")
    : fill(say(lang, "slash_effort_offered"), { levels: levels.join(" · ") });
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
          .filter(
            (each, at, all) =>
              each.model.toLowerCase().includes(typed) &&
              all.findIndex((other) => other.model === each.model) === at,
          )
          .map((each) => ({
            id: `${COMPLETED.model} ${each.model}`,
            label: each.model,
            secondary: each.label,
          })),
      };
    case COMPLETED.effort:
      return offers.levels.length === 0
        ? null
        : {
            id: COMMANDS,
            label: "talk_column_effort",
            rows: offers.levels
              .filter((level) => level.startsWith(typed))
              .map((level) => ({
                id: `${COMPLETED.effort} ${level}`,
                label: level,
                secondary: say(lang, `effort_note_${level}`),
              })),
          };
    default:
      return null;
  }
}

export function completedAt(line: string, pointed: string | undefined): string {
  return pointed !== undefined && /\s/.test(pointed) ? `${pointed} ` : completed(line, pointed);
}

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

export interface Reach extends SessionHands {
  readonly command: (command: Command) => boolean;
  readonly go: (view: View) => void;
  readonly here: Address | null;
  readonly live: RunBelief | undefined;
  readonly belief: Belief;
  readonly buildings: readonly BuildingProgress[] | undefined;
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

export type BoxKey = "complete" | "menu" | "recall" | "send" | "type";

export interface KeyPressed {
  readonly key: string;
  readonly shiftKey: boolean;
  readonly isComposing: boolean;
}

export interface BoxState {
  readonly menu: boolean;
  readonly empty: boolean;
}

export function boxKey(pressed: KeyPressed, box: BoxState): BoxKey {
  if (pressed.isComposing) return "type";
  if (box.menu) return pressed.key === "Tab" ? "complete" : "menu";
  if (pressed.key === "ArrowUp" && box.empty) return "recall";
  if (pressed.key === "Enter" && !pressed.shiftKey) return "send";
  return "type";
}

export interface ComposerLook {
  readonly form: HTMLFormAttributes;
  readonly over: boolean;
  readonly box: HTMLTextareaAttributes;

  readonly menu: Snippet | undefined;

  readonly door: Snippet | undefined;

  readonly band: Snippet | undefined;

  readonly keys: Snippet;

  readonly line: Snippet;

  readonly under: Snippet;
}
