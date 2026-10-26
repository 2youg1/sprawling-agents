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
import type { Key, Lang } from "../../core/lang";
import type { ModelFact } from "../../core/probed";
import { InputKinds as InputKindsSchema } from "../../wire";
import type { FigureFrom, FiguresLine, InputKinds } from "../../wire";
import type { Min } from "../parts/table";
import {
  ceilingOf,
  everyRow,
  isNamed,
  priceOf,
  shownRows,
  tickedRows,
  windowOf,
} from "./model_rows";
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

// The sentence under a figure box for the rung that stated it.
function fromKey(from: FigureFrom): Key {
  switch (from) {
    case "person":
      return "setup_figure_person";
    case "upstream":
      return "setup_figure_upstream";
    case "registered":
      return "setup_figure_registered";
    case "preset":
      return "setup_figure_preset";
    case "policy":
      return "setup_figure_policy";
  }
}

// What an untouched box shows: the figure a pick with this box empty
// registers, drawn as a placeholder rather than as a value, because a
// placeholder says "this is what the city will use" where a value would
// claim a person decided it, and under it the rung that stated it. The
// city answers both (`Query::ModelFigures`, client D103); until it has,
// the box is blank and says nothing.
interface Untouched {
  readonly placeholder: string;
  readonly source: string;
}

const UNANSWERED: Untouched = { placeholder: "", source: "" };

function windowShown(line: FiguresLine | undefined, lang: Lang): Untouched {
  if (line === undefined) return UNANSWERED;
  const window = line.window ?? null;
  return window === null
    ? { placeholder: "", source: say(lang, "setup_window_unknown") }
    : { placeholder: String(window.tokens), source: say(lang, fromKey(window.from)) };
}

function ceilingShown(line: FiguresLine | undefined, lang: Lang): Untouched {
  const ceiling = line?.ceiling ?? null;
  if (ceiling === null) return UNANSWERED;
  return ceiling === "provider_default"
    ? { placeholder: "", source: say(lang, "setup_ceiling_provider") }
    : { placeholder: String(ceiling.sent.tokens), source: say(lang, fromKey(ceiling.sent.from)) };
}

// The figure the city answered for a row, which a column orders an
// untouched row by.
function cityWindow(line: FiguresLine | undefined): number | null {
  return line?.window?.tokens ?? null;
}

function cityCeiling(line: FiguresLine | undefined): number | null {
  const ceiling = line?.ceiling ?? null;
  return ceiling === null || ceiling === "provider_default" ? null : ceiling.sent.tokens;
}

export interface BoxLook {
  readonly label: string;
  readonly value: string;
  readonly placeholder: string;
  readonly input: (value: string) => void;
}

// A figure box, with the rung its placeholder came from.
export interface FigureLook extends BoxLook {
  readonly source: string;
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
  // Present on a row the person named and the endpoint's list does not
  // carry: the city registers it on the person's word.
  readonly named: string | undefined;
  readonly window: FigureLook;
  readonly ceiling: FigureLook;
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
  readonly textOnly: {
    readonly label: string;
    readonly on: boolean;
    readonly toggle: (on: boolean) => void;
  };
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
  readonly manual: BoxLook;
}

// The whole value a look draws, built from the state the seat holds,
// the rows the provider served, the city's answer for each row (empty
// while it has not answered), and the person's language. Every press
// writes into `state`; the seat's own derivation draws the result.
export function lookOf(
  state: TableState,
  served: readonly ModelFact[],
  answered: readonly FiguresLine[],
  lang: Lang,
): ModelTableLook {
  const every = everyRow(served, state.manual);
  const shown = shownRows(state, served);
  const facts = new Map(every.map((row) => [row.id, row]));
  const city = new Map(answered.map((line) => [line.id, line]));
  const ticked = tickedRows(state, served);
  const figure =
    (pick: (row: ModelFact) => number | null) => (a: ModelRowLook, b: ModelRowLook) => {
      const left = facts.get(a.id);
      const right = facts.get(b.id);
      return bySize(
        left === undefined ? null : pick(left),
        right === undefined ? null : pick(right),
      );
    };
  const window = (row: ModelFact) => windowOf(state, row) ?? cityWindow(city.get(row.id));
  const ceiling = (row: ModelFact) => ceilingOf(state, row) ?? cityCeiling(city.get(row.id));
  const columns: readonly ColumnLook[] = [
    {
      key: "id",
      header: say(lang, "setup_model_id"),
      min: "24ch",
      compare: (a, b) => a.id.localeCompare(b.id),
    },
    {
      key: "context",
      header: say(lang, "setup_model_context"),
      min: "figure",
      compare: figure(window),
    },
    {
      key: "output",
      header: say(lang, "setup_model_output"),
      min: "figure",
      compare: figure(ceiling),
    },
    {
      key: "modalities",
      header: say(lang, "setup_model_modalities"),
      min: "12ch",
      compare: undefined,
    },
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
    count: fill(say(lang, "setup_ticked_count"), {
      ticked: String(ticked.length),
      total: String(every.length),
    }),
    table:
      every.length === 0
        ? undefined
        : {
            caption: say(lang, "setup_models"),
            columns,
            rows: shown.map((row) =>
              rowLook(state, { row, line: city.get(row.id), named: isNamed(served, row) }, lang),
            ),
            tick: {
              picked: (row) => state.ticked[row.id] === true,
              onPick: (row, on) => {
                state.ticked[row.id] = on;
              },
              allLabel: say(lang, "setup_select_all"),
              onPickAll: (on) => {
                for (const row of shown) state.ticked[row.id] = on;
              },
              allPicked: () =>
                shown.length > 0 && shown.every((row) => state.ticked[row.id] === true),
            },
            empty: say(lang, "setup_no_models"),
          },
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

// One row as the table draws it: the provider's facts, the city's
// answer for it, and whether the person named it.
interface Drawn {
  readonly row: ModelFact;
  readonly line: FiguresLine | undefined;
  readonly named: boolean;
}

function rowLook(state: TableState, { row, line, named }: Drawn, lang: Lang): ModelRowLook {
  const choice = (header: string) => ({
    label: `${header} ${row.id}`,
    placeholder: say(lang, "part_search"),
    empty: say(lang, "part_no_match"),
  });
  return {
    id: row.id,
    named: named ? say(lang, "setup_model_named") : undefined,
    window: {
      label: `${say(lang, "setup_model_context")} ${row.id}`,
      value: state.context[row.id] ?? "",
      ...windowShown(line, lang),
      input: (value) => {
        state.context[row.id] = value;
      },
    },
    ceiling: {
      label: `${say(lang, "setup_model_output")} ${row.id}`,
      value: state.output[row.id] ?? "",
      ...ceilingShown(line, lang),
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
