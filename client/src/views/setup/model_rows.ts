// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The probed rows as the model table reads them: which are worth
// offering, what the person has typed and ticked, and what each ticked
// row is handed back as. A probe of an endpoint that serves two hundred
// rows, most of them video and speech, is a list somebody filters, so
// the filter is here and the drawing is `./model_table`.

import type { ModelFact } from "../../core/probed";
import type { InputKinds, ModelTag } from "../../wire";
import type { ModelRow } from "./models";

// Word fragments that mark a model as something other than text in,
// text out. A heuristic over an id, used only for the rows whose own
// answer said nothing: a provider that states its input modalities is
// believed, and a name that looks like a video model is not overruled
// by a guess. It hides rows rather than refusing them, and the filter
// is a switch a person can turn off.
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

// What the person has typed and ticked, keyed by model id. The seat
// holds one of these as state and the presses below write into it, so
// the binding is never reassigned.
export interface TableState {
  readonly ticked: Record<string, boolean>;
  readonly context: Record<string, string>;
  readonly output: Record<string, string>;
  readonly role: Record<string, string>;
  readonly input: Record<string, InputKinds | null>;
  search: string;
  textOnly: boolean;
  manual: string;
}

export function freshTable(): TableState {
  return { ticked: {}, context: {}, output: {}, role: {}, input: {}, search: "", textOnly: true, manual: "" };
}

function looksTextual(row: ModelFact): boolean {
  if (row.inputModalities.length > 0) return row.inputModalities.includes("text");
  const lower = row.id.toLowerCase();
  return !NOT_TEXT.some((mark) => lower.includes(mark));
}

// A whole positive number, or nothing. An empty box and a box holding
// letters both mean "nobody stated this", which is what the wire calls
// absent.
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
      // Anything else is the box a person left alone: no role.
      return null;
  }
}

// The two prices as the provider printed them, in and out. A row that
// priced neither shows the same dash every unstated figure shows.
// wording-ok: the dash is punctuation - it stands for a figure the
// provider never printed, and reads the same in either language.
export function priceOf(row: ModelFact): string {
  if (row.inputPrice === null && row.outputPrice === null) return "—";
  return `${row.inputPrice ?? "—"} / ${row.outputPrice ?? "—"}`;
}

// The ids the person typed for an endpoint that serves no list, or
// serves one this city could not read.
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

// Every row the table knows: what the provider served, then what the
// person typed, each id once.
export function everyRow(served: readonly ModelFact[], manual: string): readonly ModelFact[] {
  const all = [...served, ...typed(manual)];
  return all.filter((row, at) => all.findIndex((other) => other.id === row.id) === at);
}

// The rows the table draws, ordered by id, which stands one vendor's
// rows together because the vendor is the part of an id before the
// slash. Any column that can be ordered by reorders it from there; this
// is where it opens. A row the person typed is never hidden by the
// text filter, because typing it was the statement that it is wanted.
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

// The figures a row is called with: what the person typed, and
// otherwise what the provider stated. An empty box therefore means
// "keep what the provider stated" rather than "state nothing", which is
// why that figure is drawn as a placeholder and not as a value.
export function windowOf(state: TableState, row: ModelFact): number | null {
  return stated(state.context[row.id] ?? "") ?? row.contextTokens;
}

export function ceilingOf(state: TableState, row: ModelFact): number | null {
  return stated(state.output[row.id] ?? "") ?? row.maxOutputTokens;
}

export function tickedRows(state: TableState, served: readonly ModelFact[]): readonly ModelFact[] {
  return everyRow(served, state.manual).filter((row) => state.ticked[row.id] === true);
}

// What the table hands back: one row per ticked model, with the figures
// it is to be called with and the role it is to fill.
export function chosenRows(state: TableState, served: readonly ModelFact[]): readonly ModelRow[] {
  return tickedRows(state, served).map((row) => ({
    id: row.id,
    stated: { contextTokens: windowOf(state, row), maxOutputTokens: ceilingOf(state, row), input: state.input[row.id] ?? null },
    tag: tagOf(state.role[row.id] ?? ""),
  }));
}
