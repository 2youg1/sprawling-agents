// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { fill, say } from "../../core/lang";
import type { Key, Lang } from "../../core/lang";
import type { ModelFact } from "../../core/probed";
import { InputKinds as InputKindsSchema } from "../../wire";
import type { FigureFrom, FiguresLine, InputKinds, ModelTag } from "../../wire";
import type { Min } from "../parts/table";
import {
  ceilingOf,
  everyRow,
  isNamed,
  priceOf,
  setTick,
  shownRows,
  tickedRows,
  windowOf,
} from "./model_rows";
import type { TableState } from "./model_rows";

const INPUTS: readonly InputKinds[] = InputKindsSchema.literals;

const MANUAL_PLACEHOLDER = "gpt-5-mini, claude-sonnet-4";

function bySize(a: number | null, b: number | null): number {
  return (a ?? 0) - (b ?? 0);
}

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

export interface ModelRowLook {
  readonly id: string;
  readonly named: string | undefined;
  readonly window: FigureLook;
  readonly ceiling: FigureLook;
  readonly input: ChoiceLook;
  readonly said: string;
  readonly price: string;
  readonly role: ChoiceLook;
  readonly refusal: string | undefined;
}

export type ColumnKey = "id" | "context" | "output" | "modalities" | "price" | "role";

export interface ColumnLook {
  readonly key: ColumnKey;
  readonly header: string;
  readonly min: Min;
  readonly compare: ((a: ModelRowLook, b: ModelRowLook) => number) | undefined;
}

export interface TickLook {
  readonly picked: (row: ModelRowLook) => boolean;
  readonly onPick: (row: ModelRowLook, on: boolean) => void;
  readonly allLabel: string;
  readonly onPickAll: (on: boolean) => void;
  readonly allPicked: () => boolean;
}

export function roleKey(role: ModelTag): Key {
  switch (role) {
    case "main":
      return "setup_main";
    case "digest":
      return "setup_digest";
    case "transcribe":
      return "setup_transcribe";
    case "ocr":
      return "setup_ocr";
  }
}

export interface ModelTableLook {
  readonly search: BoxLook;
  readonly textOnly: {
    readonly label: string;
    readonly on: boolean;
    readonly toggle: (on: boolean) => void;
  };
  readonly count: string;
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
                setTick(state, row.id, on);
              },
              allLabel: say(lang, "setup_select_all"),
              onPickAll: (on) => {
                for (const row of shown) setTick(state, row.id, on);
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
    input: {
      ...choice(say(lang, "setup_model_modalities")),
      placeholder:
        line === undefined ? say(lang, "part_search") : say(lang, `setup_input_${line.input}`),
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
        state.refusal = null;
        if (value !== "") {
          for (const id of Object.keys(state.role)) {
            if (id !== row.id && state.role[id] === value) state.role[id] = "";
          }
        }
        state.role[row.id] = value;
      },
    },
    refusal:
      state.refusal?.id === row.id
        ? fill(say(lang, "setup_model_role_taken"), {
            id: row.id,
            role: say(lang, roleKey(state.refusal.role)),
          })
        : undefined,
  };
}
