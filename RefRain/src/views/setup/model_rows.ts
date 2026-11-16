// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { providerName, wireCeiling, wireWindow } from "../../core/commands";
import type { ModelFact } from "../../core/probed";
import type { DialectKind, FiguresAsked, InputKinds, ModelTag, Query } from "../../wire";
import type { ModelRow } from "./models";

const NOT_TEXT: readonly string[] = [
  "asr",
  "audio",
  "diffusion",
  "embed",
  "image",
  "imagen",
  "flux",
  "rerank",
  "sora",
  "speech",
  "tts",
  "veo",
  "video",
  "vision",
  "whisper",
];

export interface TableState {
  readonly ticked: Record<string, boolean>;
  readonly context: Record<string, string>;
  readonly output: Record<string, string>;
  role: Record<string, string>;
  heldRole: Record<string, ModelTag>;
  readonly input: Record<string, InputKinds | null>;
  refusal: { readonly id: string; readonly role: ModelTag } | null;
  search: string;
  textOnly: boolean;
  manual: string;
}

export interface TableSeed {
  readonly ticked?: readonly string[];
  readonly named?: readonly string[];
  readonly roles?: Readonly<Record<string, ModelTag>>;
}

export function freshTable(seed: TableSeed = {}): TableState {
  const roles = seed.roles ?? {};
  return {
    ticked: Object.fromEntries((seed.ticked ?? []).map((id) => [id, true])),
    context: {},
    output: {},
    role: { ...roles },
    heldRole: { ...roles },
    input: {},
    refusal: null,
    search: "",
    textOnly: true,
    manual: (seed.named ?? []).join(", "),
  };
}

export function setTick(state: TableState, id: string, on: boolean): void {
  const held = state.heldRole[id];
  if (!on && held !== undefined) {
    state.refusal = { id, role: held };
    return;
  }
  state.refusal = null;
  state.ticked[id] = on;
}

function looksTextual(row: ModelFact): boolean {
  if (row.inputModalities.length > 0) return row.inputModalities.includes("text");
  const lower = row.id.toLowerCase();
  return !NOT_TEXT.some((mark) => lower.includes(mark));
}

function stated(text: string): number | null {
  const trimmed = text.trim();
  if (trimmed === "" || !/^[0-9]+$/.test(trimmed)) return null;
  const figure = Number.parseInt(trimmed, 10);
  return figure > 0 ? figure : null;
}

function tagOf(value: string): ModelTag | null {
  switch (value) {
    case "main":
    case "digest":
    case "transcribe":
      return value;
    default:
      return null;
  }
}

export function priceOf(row: ModelFact): string {
  if (row.inputPrice === null && row.outputPrice === null) return "—";
  return `${row.inputPrice ?? "—"} / ${row.outputPrice ?? "—"}`;
}

function typed(manual: string): readonly ModelFact[] {
  return manual
    .split(",")
    .map((each) => each.trim())
    .filter((each) => each !== "")
    .map((id) => ({
      id,
      contextTokens: null,
      maxOutputTokens: null,
      inputModalities: [],
      inputPrice: null,
      outputPrice: null,
    }));
}

export function everyRow(served: readonly ModelFact[], manual: string): readonly ModelFact[] {
  const all = [...served, ...typed(manual)];
  return all.filter((row, at) => all.findIndex((other) => other.id === row.id) === at);
}

export function shownRows(state: TableState, served: readonly ModelFact[]): readonly ModelFact[] {
  const needle = state.search.trim().toLowerCase();
  const byHand = typed(state.manual).map((row) => row.id);
  return everyRow(served, state.manual)
    .filter(
      (row) =>
        (needle === "" || row.id.toLowerCase().includes(needle)) &&
        (!state.textOnly || looksTextual(row) || byHand.includes(row.id)),
    )
    .sort((a, b) => a.id.localeCompare(b.id));
}

export function windowOf(state: TableState, row: ModelFact): number | null {
  return stated(state.context[row.id] ?? "");
}

export function ceilingOf(state: TableState, row: ModelFact): number | null {
  return stated(state.output[row.id] ?? "");
}

export function isNamed(served: readonly ModelFact[], row: ModelFact): boolean {
  return !served.some((each) => each.id === row.id);
}

export interface TableAbout {
  readonly endpoint: string;
  readonly baseUrl: string;
  readonly face: DialectKind;
}

export function figuresQuery(
  about: TableAbout,
  state: TableState,
  served: readonly ModelFact[],
): Query {
  const models: FiguresAsked[] = everyRow(served, state.manual).map((row) => ({
    id: row.id,
    context_tokens: wireWindow(row.contextTokens),
    max_output_tokens: wireCeiling(row.maxOutputTokens),
    tag: tagOf(state.role[row.id] ?? ""),
  }));
  return {
    model_figures: {
      endpoint: providerName(about.endpoint),
      base_url: about.baseUrl,
      face: about.face,
      models,
    },
  };
}

export function tickedRows(state: TableState, served: readonly ModelFact[]): readonly ModelFact[] {
  return everyRow(served, state.manual).filter((row) => state.ticked[row.id] === true);
}

export function chosenRows(state: TableState, served: readonly ModelFact[]): readonly ModelRow[] {
  return tickedRows(state, served).map((row) => ({
    id: row.id,
    stated: {
      contextTokens: windowOf(state, row),
      maxOutputTokens: ceilingOf(state, row),
      input: state.input[row.id] ?? null,
    },
    tag: tagOf(state.role[row.id] ?? ""),
  }));
}
