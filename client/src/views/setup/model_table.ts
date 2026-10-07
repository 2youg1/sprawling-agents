// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The wiring of the probed-model table: which rows are worth offering,
// what each figure means, which role a row fills, and the whole value a
// look draws (`ModelTableLook`). Nothing here touches the DOM or uses a
// rune, so the wiring test drives it with plain objects and no look,
// and another look - one built from a component library - draws the
// same table by taking the same value (client D93's cut, applied to a
// screen of its own).

import { fill, say } from "../../core/lang";
import type { Lang } from "../../core/lang";
import type { ModelFact } from "../../core/probed";
import { InputKinds as InputKindsSchema } from "../../wire";
import type { InputKinds, ModelTag } from "../../wire";
import type { Min } from "../parts/table";
import type { ModelRow } from "./models";

// The input kinds a person may state, in the wire's own order.
const INPUTS: readonly InputKinds[] = InputKindsSchema.literals;

// wording-ok: model ids, which are the same letters in every language.
const MANUAL_PLACEHOLDER = "gpt-5-mini, claude-sonnet-4";

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

export function looksTextual(row: ModelFact): boolean {
  if (row.inputModalities.length > 0) return row.inputModalities.includes("text");
  const lower = row.id.toLowerCase();
  return !NOT_TEXT.some((mark) => lower.includes(mark));
}

// A whole positive number, or nothing. An empty box and a box holding
// letters both mean "nobody stated this", which is what the wire calls
// absent.
export function stated(text: string): number | null {
  const trimmed = text.trim();
  if (trimmed === "" || !/^[0-9]+$/.test(trimmed)) return null;
  const figure = Number.parseInt(trimmed, 10);
  return figure > 0 ? figure : null;
}

export function tagOf(value: string): ModelTag | null {
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

// A row that states no figure sorts below every row that does, because
// the question this column is ordered to answer is which of them is the
// biggest.
function bySize(a: number | null, b: number | null): number {
  return (a ?? 0) - (b ?? 0);
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
function windowOf(state: TableState, row: ModelFact): number | null {
  return stated(state.context[row.id] ?? "") ?? row.contextTokens;
}

function ceilingOf(state: TableState, row: ModelFact): number | null {
  return stated(state.output[row.id] ?? "") ?? row.maxOutputTokens;
}

function tickedRows(state: TableState, served: readonly ModelFact[]): readonly ModelFact[] {
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

// What an untouched box shows: the figure the provider itself stated,
// drawn as a placeholder rather than as a value, because a placeholder
// says "this is what will be sent" where a value would claim a person
// decided it. A provider that stated nothing leaves the box blank.
function placeholderOf(figure: number | null): string {
  return figure === null ? "" : String(figure);
}

export interface BoxLook {
  readonly label: string;
  readonly value: string;
  readonly placeholder: string;
  readonly input: (value: string) => void;
}

export interface ChoiceLook {
  readonly label: string;
  readonly placeholder: string;
  readonly empty: string;
  readonly choices: readonly { readonly value: string; readonly label: string }[];
  readonly value: string | null;
  readonly pick: (value: string) => void;
}

// One drawn row of the table: every control it holds, already wired.
export interface ModelRowLook {
  readonly id: string;
  readonly window: BoxLook;
  readonly ceiling: BoxLook;
  readonly input: ChoiceLook;
  // The input modalities the provider itself stated, spelled as the
  // provider spelled them; empty when it stated none.
  readonly said: string;
  readonly price: string;
  readonly role: ChoiceLook;
}

export type ColumnKey = "id" | "context" | "output" | "modalities" | "price" | "role";

export interface ColumnLook {
  readonly key: ColumnKey;
  readonly header: string;
  readonly min: Min;
  // Present makes the column sortable.
  readonly compare: ((a: ModelRowLook, b: ModelRowLook) => number) | undefined;
}

export interface TickLook {
  readonly picked: (row: ModelRowLook) => boolean;
  readonly onPick: (row: ModelRowLook, on: boolean) => void;
  readonly allLabel: string;
  readonly onPickAll: (on: boolean) => void;
  readonly allPicked: () => boolean;
}

export interface ModelTableLook {
  readonly search: BoxLook;
  readonly textOnly: { readonly label: string; readonly on: boolean; readonly toggle: (on: boolean) => void };
  readonly count: string;
  // Absent while there is no row at all, because an empty table under
  // an empty count says nothing the count did not.
  readonly table:
    | {
        readonly caption: string;
        readonly columns: readonly ColumnLook[];
        readonly rows: readonly ModelRowLook[];
        readonly tick: TickLook;
        readonly empty: string;
      }
    | undefined;
  // Present while a ticked row has no output ceiling: not an alarm. The
  // ceiling ladder in the city always answers
  // (`gateway::OutputCeiling::resolve`), so an empty box is a decision
  // left to the city rather than a call that will be refused, and the
  // line says which rungs will answer it.
  readonly needed: string | undefined;
  readonly manual: BoxLook;
}

// The whole value a look draws, built from the state the seat holds,
// the rows the provider served, and the person's language. Every press
// writes into `state`; the seat's own derivation draws the result.
export function lookOf(state: TableState, served: readonly ModelFact[], lang: Lang): ModelTableLook {
  const every = everyRow(served, state.manual);
  const shown = shownRows(state, served);
  const facts = new Map(every.map((row) => [row.id, row]));
  const ticked = tickedRows(state, served);
  const figure = (pick: (row: ModelFact) => number | null) => (a: ModelRowLook, b: ModelRowLook) => {
    const left = facts.get(a.id);
    const right = facts.get(b.id);
    return bySize(left === undefined ? null : pick(left), right === undefined ? null : pick(right));
  };
  const columns: readonly ColumnLook[] = [
    { key: "id", header: say(lang, "setup_model_id"), min: "24ch", compare: (a, b) => a.id.localeCompare(b.id) },
    { key: "context", header: say(lang, "setup_model_context"), min: "figure", compare: figure((row) => windowOf(state, row)) },
    { key: "output", header: say(lang, "setup_model_output"), min: "figure", compare: figure((row) => ceilingOf(state, row)) },
    { key: "modalities", header: say(lang, "setup_model_modalities"), min: "12ch", compare: undefined },
    { key: "price", header: say(lang, "setup_model_price"), min: "12ch", compare: undefined },
    { key: "role", header: say(lang, "setup_model_role"), min: "figure", compare: undefined },
  ];
  return {
    search: {
      label: say(lang, "setup_model_search"),
      value: state.search,
      placeholder: say(lang, "setup_model_search"),
      input: (value) => {
        state.search = value;
      },
    },
    textOnly: {
      label: say(lang, "setup_text_only"),
      on: state.textOnly,
      toggle: (on) => {
        state.textOnly = on;
      },
    },
    count: fill(say(lang, "setup_ticked_count"), { ticked: String(ticked.length), total: String(every.length) }),
    table:
      every.length === 0
        ? undefined
        : {
            caption: say(lang, "setup_models"),
            columns,
            rows: shown.map((row) => rowLook(state, row, lang)),
            tick: {
              picked: (row) => state.ticked[row.id] === true,
              onPick: (row, on) => {
                state.ticked[row.id] = on;
              },
              allLabel: say(lang, "setup_select_all"),
              onPickAll: (on) => {
                for (const row of shown) state.ticked[row.id] = on;
              },
              allPicked: () => shown.length > 0 && shown.every((row) => state.ticked[row.id] === true),
            },
            empty: say(lang, "setup_no_models"),
          },
    needed: ticked.some((row) => ceilingOf(state, row) === null) ? say(lang, "setup_model_needed") : undefined,
    manual: {
      label: say(lang, "setup_manual_ids"),
      value: state.manual,
      placeholder: MANUAL_PLACEHOLDER,
      input: (value) => {
        state.manual = value;
      },
    },
  };
}

function rowLook(state: TableState, row: ModelFact, lang: Lang): ModelRowLook {
  const choice = (header: string) => ({
    label: `${header} ${row.id}`,
    placeholder: say(lang, "part_search"),
    empty: say(lang, "part_no_match"),
  });
  return {
    id: row.id,
    window: {
      label: `${say(lang, "setup_model_context")} ${row.id}`,
      value: state.context[row.id] ?? "",
      placeholder: placeholderOf(row.contextTokens),
      input: (value) => {
        state.context[row.id] = value;
      },
    },
    ceiling: {
      label: `${say(lang, "setup_model_output")} ${row.id}`,
      value: state.output[row.id] ?? "",
      placeholder: placeholderOf(row.maxOutputTokens),
      input: (value) => {
        state.output[row.id] = value;
      },
    },
    // What the model takes as input, as the person states it (X6): a
    // model that reads pictures is the one a picture may be sent to, and
    // an unstated row is the city's to look up. The same choice control
    // as the role beside it, so one row offers one kind of choice.
    input: {
      ...choice(say(lang, "setup_model_modalities")),
      choices: [
        { value: "", label: say(lang, "setup_input_unstated") },
        ...INPUTS.map((each) => ({ value: each, label: say(lang, `setup_input_${each}`) })),
      ],
      value: state.input[row.id] ?? "",
      pick: (value) => {
        state.input[row.id] = INPUTS.find((each) => each === value) ?? null;
      },
    },
    said: row.inputModalities.join(" "),
    price: priceOf(row),
    // The three roles, and the one that takes a role back off a row.
    role: {
      ...choice(say(lang, "setup_model_role")),
      choices: [
        { value: "", label: say(lang, "setup_role_none") },
        { value: "main", label: say(lang, "setup_main") },
        { value: "digest", label: say(lang, "setup_digest") },
        { value: "transcribe", label: say(lang, "setup_transcribe") },
      ],
      value: state.role[row.id] ?? null,
      pick: (value) => {
        state.role[row.id] = value;
      },
    },
  };
}
