// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The wiring of the probed-model table: the whole value a look draws
// (`ModelTableLook`), built over the rows `./model_rows` offers - every
// word translated, every column's ordering, and what each press writes.
// Nothing here touches the DOM or uses a rune, so the wiring test drives
// it with plain objects and no look, and another look - one built from a
// component library - draws the same table by taking the same value
// (client D93's cut, applied to a screen of its own).

import { fill, say } from "../../core/lang";
import type { Lang } from "../../core/lang";
import type { ModelFact } from "../../core/probed";
import { InputKinds as InputKindsSchema } from "../../wire";
import type { InputKinds } from "../../wire";
import type { Min } from "../parts/table";
import { ceilingOf, everyRow, priceOf, shownRows, tickedRows, windowOf } from "./model_rows";
import type { TableState } from "./model_rows";

// The input kinds a person may state, in the wire's own order.
const INPUTS: readonly InputKinds[] = InputKindsSchema.literals;

// wording-ok: model ids, which are the same letters in every language.
const MANUAL_PLACEHOLDER = "gpt-5-mini, claude-sonnet-4";

// A row that states no figure sorts below every row that does, because
// the question this column is ordered to answer is which of them is the
// biggest.
function bySize(a: number | null, b: number | null): number {
  return (a ?? 0) - (b ?? 0);
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
