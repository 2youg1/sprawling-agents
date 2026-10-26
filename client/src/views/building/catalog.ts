// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The properties this module must hold are proved in `client/spec/Views/Catalog.lean`.
//
// The building page's capabilities section (client/Spec.lean 4-50, its
// tenth part): the draft of one `[catalog]` table, the exposure each
// tool takes under it, the bytes a save would add or take from every
// request, and the value `catalog.look.svelte` draws. The page edits one
// layer at a time, this building's or the city's, and saves that layer's
// whole table (`crates/wire/Spec.lean` §8-99).
//
// A row's exposure is read the way the city reads it
// (`kernel::config::CatalogExposure::stated`): a fixed tool is core; a
// nearer layer's statement wins over the draft; then the draft's tool
// key, then its namespace key, then what the row inherits from the
// layers farther than the one being edited. `rowsOf` works out the
// inherited and the nearer values from the city's answer, so the order
// is spelled here once more only for the draft the city has not seen.

import type { Key, Lang } from "../../core/lang";
import { fill, say } from "../../core/lang";
import { ago, count } from "../../core/time";
import type { CatalogAnswer, CatalogTool, Exposure, ExposureFrom, ExposureTable } from "../../wire";
import type { Choice } from "../parts/segmented";

// Which layer a save writes.
export type Scope = "building" | "city";

// One layer's table as the page edits it: both maps always present.
export interface Table {
  readonly tools: Readonly<Record<string, Exposure>>;
  readonly namespaces: Readonly<Record<string, Exposure>>;
}

// One tool as the draft reads it.
export interface Row {
  readonly tool: CatalogTool;
  readonly fixed: boolean;
  // What the row is when the edited layer states nothing about it.
  readonly inherited: Exposure;
  // What a layer nearer than the edited one states, which wins.
  readonly nearer: Exposure | undefined;
}

export const EXPOSURES: readonly Exposure[] = ["core", "dormant", "off"];

export function tableOf(table: ExposureTable): Table {
  return { tools: { ...table.tools }, namespaces: { ...table.namespaces } };
}

// The draft a scope starts from: that layer's table as written.
export function draftOf(answer: CatalogAnswer, scope: Scope): Table {
  return tableOf(scope === "building" ? answer.stated : answer.city);
}

// The rows of the answer, read for the layer being edited.
export function rowsOf(answer: CatalogAnswer, scope: Scope): readonly Row[] {
  const city = tableOf(answer.city);
  const building = tableOf(answer.stated);
  return answer.tools.map((tool) => {
    const said = (table: Table): Exposure | undefined =>
      table.tools[tool.name] ?? table.namespaces[tool.namespace];
    return {
      tool,
      fixed: tool.from === "fixed",
      inherited: scope === "building" ? (said(city) ?? tool.fallback) : tool.fallback,
      nearer: scope === "building" ? undefined : said(building),
    };
  });
}

// The row's exposure with its tool key left out of the draft.
function without(table: Table, row: Row): Exposure {
  return table.namespaces[row.tool.namespace] ?? row.inherited;
}

// The row's exposure under the draft.
export function effectiveOf(table: Table, row: Row): Exposure {
  if (row.fixed) return "core";
  if (row.nearer !== undefined) return row.nearer;
  return table.tools[row.tool.name] ?? without(table, row);
}

function omit(record: Readonly<Record<string, Exposure>>, key: string): Record<string, Exposure> {
  return Object.fromEntries(Object.entries(record).filter(([held]) => held !== key));
}

// One press on a row: the tool key is written only when it departs from
// what the row would be without it, so setting a row back to what it
// inherits takes the key out rather than writing the same value again.
export function pressRow(table: Table, row: Row, value: Exposure): Table {
  if (row.fixed) return table;
  const tools = omit(table.tools, row.tool.name);
  return {
    ...table,
    tools: value === without(table, row) ? tools : { ...tools, [row.tool.name]: value },
  };
}

// One press on a namespace: every member's tool key goes, and the
// namespace key is written unless every member inherits that value.
export function pressGroup(
  rows: readonly Row[],
  table: Table,
  namespace: string,
  value: Exposure,
): Table {
  const members = rows.filter((row) => row.tool.namespace === namespace && !row.fixed);
  const tools = members.reduce((held, row) => omit(held, row.tool.name), { ...table.tools });
  const namespaces = members.every((row) => row.inherited === value)
    ? omit(table.namespaces, namespace)
    : { ...table.namespaces, [namespace]: value };
  return { tools, namespaces };
}

// The bytes every request gains under the draft against the saved
// table: what moves into the core, less what moves out of it.
export function meterOf(saved: Table, draft: Table, rows: readonly Row[]): number {
  return rows.reduce((sum, row) => {
    const now = effectiveOf(draft, row) === "core" ? row.tool.bytes : 0;
    const was = effectiveOf(saved, row) === "core" ? row.tool.bytes : 0;
    return sum + now - was;
  }, 0);
}

// Whether two tables state the same thing.
export function sameTable(left: Table, right: Table): boolean {
  return canonical(left) === canonical(right);
}

function canonical(table: Table): string {
  const sorted = (record: Readonly<Record<string, Exposure>>): [string, Exposure][] =>
    Object.entries(record).sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0));
  return JSON.stringify([sorted(table.tools), sorted(table.namespaces)]);
}

const EXPOSURE_WORD: Record<Exposure, Key> = {
  core: "catalog_core",
  dormant: "catalog_dormant",
  off: "catalog_off",
};

const FROM_WORD: Record<ExposureFrom, Key> = {
  fixed: "catalog_from_fixed",
  default: "catalog_from_default",
  city: "catalog_from_city",
  building: "catalog_from_building",
};

// Everything `catalog.look.svelte` draws, already worded by the seat.
export interface CatalogLook {
  readonly scopeLabel: string;
  readonly scopes: readonly Choice<Scope>[];
  readonly scope: Scope;
  readonly onScope: (scope: Scope) => void;
  // The bytes the newest run carried, and what the draft would change.
  readonly meter: string;
  readonly delta: string | undefined;
  // Said instead of the rows before the building's first run.
  readonly empty: string | undefined;
  readonly groups: readonly GroupLook[];
  readonly save: {
    readonly label: string;
    readonly why: string | undefined;
    readonly onPress: () => void;
  };
  readonly note: string;
}

export interface GroupLook {
  readonly key: string;
  readonly title: string;
  readonly summary: string;
  readonly open: boolean;
  readonly choices: readonly Choice<Exposure>[];
  // The value every member holds, or `null` when they differ.
  readonly held: Exposure | null;
  readonly onPick: (value: Exposure) => void;
  readonly rows: readonly RowLook[];
}

export interface RowLook {
  readonly key: string;
  readonly name: string;
  readonly hint: string;
  readonly label: string;
  readonly choices: readonly Choice<Exposure>[];
  readonly held: Exposure;
  readonly onPick: (value: Exposure) => void;
  readonly facts: string;
}

// What the seat holds and does.
export interface Seat {
  readonly answer: CatalogAnswer;
  readonly scope: Scope;
  readonly draft: Table;
  readonly edited: boolean;
  readonly now: number;
}

export interface Hands {
  readonly scope: (scope: Scope) => void;
  readonly edit: (draft: Table) => void;
  readonly save: () => void;
}

// A group draws open when it is short enough to read at a glance.
const OPEN_GROUP = 12;

export function catalogLook(seat: Seat, lang: Lang, hands: Hands): CatalogLook {
  const rows = rowsOf(seat.answer, seat.scope);
  const saved = draftOf(seat.answer, seat.scope);
  const delta = meterOf(saved, seat.draft, rows);
  const choices = (why: string | undefined): readonly Choice<Exposure>[] =>
    EXPOSURES.map((value) => ({
      value,
      label: say(lang, EXPOSURE_WORD[value]),
      ...(why === undefined ? {} : { why }),
    }));
  const namespaces = [...new Set(rows.map((row) => row.tool.namespace))];
  return {
    scopeLabel: say(lang, "catalog_scope"),
    scopes: [
      { value: "building", label: say(lang, "catalog_scope_building") },
      { value: "city", label: say(lang, "catalog_scope_city") },
    ],
    scope: seat.scope,
    onScope: hands.scope,
    meter: fill(say(lang, "catalog_meter"), {
      core: count(seat.answer.core_bytes),
      index: count(seat.answer.index_bytes),
    }),
    delta:
      seat.edited && delta !== 0
        ? fill(say(lang, delta > 0 ? "catalog_more" : "catalog_less"), {
            n: count(Math.abs(delta)),
          })
        : undefined,
    empty: seat.answer.run == null ? say(lang, "catalog_no_run") : undefined,
    groups: namespaces.map((namespace) => {
      const members = rows.filter((row) => row.tool.namespace === namespace);
      const held = members.map((row) => effectiveOf(seat.draft, row));
      const tally = EXPOSURES.map((value) => held.filter((each) => each === value).length);
      return {
        key: namespace,
        title: namespace,
        summary: fill(say(lang, "catalog_group"), {
          n: String(members.length),
          core: String(tally[0] ?? 0),
          dormant: String(tally[1] ?? 0),
          off: String(tally[2] ?? 0),
        }),
        open: members.length <= OPEN_GROUP,
        choices: choices(
          members.every((row) => row.fixed) ? say(lang, "catalog_fixed") : undefined,
        ),
        held: held.every((each) => each === held[0]) ? (held[0] ?? null) : null,
        onPick: (value) => {
          hands.edit(pressGroup(rows, seat.draft, namespace, value));
        },
        rows: members.map((row) => rowLook(row, seat, lang, { choices, edit: hands.edit })),
      };
    }),
    save: {
      label: say(lang, seat.scope === "building" ? "catalog_save_building" : "catalog_save_city"),
      why: seat.edited ? undefined : say(lang, "catalog_unchanged"),
      onPress: hands.save,
    },
    note: say(lang, "catalog_next_run"),
  };
}

function rowLook(
  row: Row,
  seat: Seat,
  lang: Lang,
  hands: {
    readonly choices: (why: string | undefined) => readonly Choice<Exposure>[];
    readonly edit: (draft: Table) => void;
  },
): RowLook {
  const last = row.tool.last_used;
  const used =
    row.tool.runs_used > 0 && last != null
      ? fill(say(lang, "catalog_used"), {
          n: String(row.tool.runs_used),
          of: String(seat.answer.runs_seen),
          when: ago(lang, last, seat.now),
        })
      : fill(say(lang, "catalog_unused"), { of: String(seat.answer.runs_seen) });
  return {
    key: row.tool.name,
    name: row.tool.name,
    hint: row.tool.hint,
    label: fill(say(lang, "catalog_row"), { name: row.tool.name }),
    choices: hands.choices(row.fixed ? say(lang, "catalog_fixed") : undefined),
    held: effectiveOf(seat.draft, row),
    onPick: (value) => {
      hands.edit(pressRow(seat.draft, row, value));
    },
    facts: [
      say(lang, FROM_WORD[row.tool.from]),
      used,
      fill(say(lang, "run_prompt_bytes"), { n: count(row.tool.bytes) }),
    ].join(" · "),
  };
}
