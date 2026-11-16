// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


import type { Key } from "../../core/lang";
import type {
  B3Hash,
  CatalogAnswer,
  CatalogChange,
  CatalogSkill,
  CatalogTool,
  Exposure,
  ExposureFrom,
  ExposureTable,
  McpReference,
} from "../../wire";

export type Scope = "building" | "city";

export interface Table {
  readonly tools: Readonly<Record<string, Exposure>>;
  readonly namespaces: Readonly<Record<string, Exposure>>;
  readonly skills: Readonly<Record<string, Exposure>>;
}

export interface Selection {
  readonly catalog: Table;
  readonly references: readonly McpReference[];
  readonly skillPins: Readonly<Record<string, B3Hash>>;
  readonly readingRoom: readonly string[];
}

export interface Row {
  readonly tool: CatalogTool;
  readonly fixed: boolean;
  readonly inherited: Exposure;
  readonly nearer: Exposure | undefined;
}

export interface SkillRow {
  readonly skill: CatalogSkill;
  readonly inherited: Exposure;
  readonly nearer: Exposure | undefined;
}

export const EXPOSURES: readonly Exposure[] = ["core", "dormant", "off"];

export function tableOf(table: ExposureTable): Table {
  return {
    tools: { ...table.tools },
    namespaces: { ...table.namespaces },
    skills: { ...table.skills },
  };
}

export function draftOf(answer: CatalogAnswer, scope: Scope): Table {
  return tableOf(scope === "building" ? answer.stated : answer.city);
}

export function selectionOf(answer: CatalogAnswer): Selection {
  return {
    catalog: draftOf(answer, "building"),
    references: answer.references.map((reference) => ({
      ...reference,
      tools: reference.tools == null ? reference.tools : { ...reference.tools },
    })),
    skillPins: { ...answer.skill_pins },
    readingRoom: [...answer.reading_room],
  };
}

export function sameSelection(left: Selection, right: Selection): boolean {
  return canonicalSelection(left) === canonicalSelection(right);
}


export interface SaveSettle {
  readonly landed: boolean;
  readonly edited: boolean;
  readonly saved: boolean;
}

export function settleSave(
  held: Selection,
  draft: Selection,
  sent: Selection,
  layer: Scope,
): SaveSettle {
  const landed = sameDraft(held, sent, layer);
  if (!landed) return { landed: false, edited: false, saved: false };
  const edited = !sameDraft(draft, sent, layer);
  return { landed: true, edited, saved: !edited };
}

function sameDraft(left: Selection, right: Selection, layer: Scope): boolean {
  return layer === "building" ? sameSelection(left, right) : sameTable(left.catalog, right.catalog);
}

function canonicalSelection(selection: Selection): string {
  return JSON.stringify({
    catalog: canonical(selection.catalog),
    references: selection.references,
    pins: sorted(selection.skillPins),
    reading: selection.readingRoom,
  });
}

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

export function skillRowsOf(answer: CatalogAnswer, scope: Scope): readonly SkillRow[] {
  const city = tableOf(answer.city);
  const building = tableOf(answer.stated);
  return answer.skill_rows.map((skill) => ({
    skill,
    inherited: scope === "building" ? (city.skills[skill.name] ?? "dormant") : "dormant",
    nearer: scope === "building" ? undefined : building.skills[skill.name],
  }));
}

function omit<T>(record: Readonly<Record<string, T>>, key: string): Record<string, T> {
  return Object.fromEntries(Object.entries(record).filter(([held]) => held !== key));
}

function without(table: Table, row: Row): Exposure {
  return table.namespaces[row.tool.namespace] ?? row.inherited;
}

export function effectiveOf(table: Table, row: Row): Exposure {
  if (row.fixed) return "core";
  if (row.nearer !== undefined) return row.nearer;
  return table.tools[row.tool.name] ?? without(table, row);
}

export function pressRow(table: Table, row: Row, value: Exposure): Table {
  if (row.fixed) return table;
  const tools = omit(table.tools, row.tool.name);
  return {
    ...table,
    tools: value === without(table, row) ? tools : { ...tools, [row.tool.name]: value },
  };
}

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
  return { ...table, tools, namespaces };
}

export function effectiveSkill(table: Table, row: SkillRow, admitted: boolean): Exposure {
  if (!admitted) return "off";
  if (row.nearer !== undefined) return row.nearer;
  return table.skills[row.skill.name] ?? row.inherited;
}

export function pressSkill(
  selection: Selection,
  row: SkillRow,
  value: Exposure,
  scope: Scope,
): Selection {
  const inherited = row.inherited;
  const skills = omit(selection.catalog.skills, row.skill.name);
  const catalog = {
    ...selection.catalog,
    skills: value === inherited ? skills : { ...skills, [row.skill.name]: value },
  };
  if (scope === "city") return { ...selection, catalog };
  const readingRoom =
    value === "off"
      ? selection.readingRoom.filter((name) => name !== row.skill.name)
      : [...new Set([...selection.readingRoom, row.skill.name])];
  const skillPins =
    value !== "off" &&
    selection.skillPins[row.skill.name] === undefined &&
    row.skill.candidate != null
      ? { ...selection.skillPins, [row.skill.name]: row.skill.candidate }
      : selection.skillPins;
  return { ...selection, catalog, skillPins, readingRoom };
}

export function acceptTool(selection: Selection, tool: CatalogTool): Selection {
  return {
    ...selection,
    references: selection.references.map((reference) => {
      if (reference.label !== tool.namespace) return reference;
      const tools = { ...reference.tools };
      return {
        ...reference,
        tools:
          tool.current == null
            ? omit(tools, tool.name)
            : { ...tools, [tool.name]: tool.current.digest },
      };
    }),
  };
}

export function acceptSkill(selection: Selection, skill: CatalogSkill): Selection {
  const skillPins =
    skill.candidate == null
      ? omit(selection.skillPins, skill.name)
      : { ...selection.skillPins, [skill.name]: skill.candidate };
  return { ...selection, skillPins };
}

export function keepSkill(selection: Selection, skill: CatalogSkill): Selection {
  const skillPins = omit(selection.skillPins, skill.name);
  if (skill.pinned != null) skillPins[skill.name] = skill.pinned;
  return { ...selection, skillPins };
}

export function meterOf(saved: Table, draft: Table, rows: readonly Row[]): number {
  return rows.reduce((sum, row) => {
    const now = effectiveOf(draft, row) === "core" ? bytesOf(row.tool) : 0;
    const was = effectiveOf(saved, row) === "core" ? savedBytesOf(row.tool) : 0;
    return sum + now - was;
  }, 0);
}

export function skillMeterOf(
  saved: Selection,
  draft: Selection,
  rows: readonly SkillRow[],
): number {
  return rows.reduce((sum, row) => {
    const nowBytes = skillDocumentBytes(row.skill);
    const wasBytes = textBytes(row.skill.pinned_document ?? row.skill.current_document ?? "");
    const now = effectiveSkill(draft.catalog, row, draft.readingRoom.includes(row.skill.name));
    const was = effectiveSkill(saved.catalog, row, saved.readingRoom.includes(row.skill.name));
    return sum + (now === "core" ? nowBytes : 0) - (was === "core" ? wasBytes : 0);
  }, 0);
}

export function sameTable(left: Table, right: Table): boolean {
  return canonical(left) === canonical(right);
}

function canonical(table: Table): string {
  return JSON.stringify([sorted(table.tools), sorted(table.namespaces), sorted(table.skills)]);
}

function sorted<T>(record: Readonly<Record<string, T>>): [string, T][] {
  return Object.entries(record).sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0));
}

export const EXPOSURE_WORD: Record<Exposure, Key> = {
  core: "catalog_core",
  dormant: "catalog_dormant",
  off: "catalog_off",
};

export const FROM_WORD: Record<ExposureFrom, Key> = {
  fixed: "catalog_from_fixed",
  default: "catalog_from_default",
  city: "catalog_from_city",
  building: "catalog_from_building",
};

export const CHANGE_WORD: Record<CatalogChange, Key> = {
  added: "catalog_added",
  changed: "catalog_changed",
  removed: "catalog_removed",
};



export function bytesOf(tool: CatalogTool): number {
  return tool.current?.bytes ?? tool.pinned?.bytes ?? 0;
}

export function savedBytesOf(tool: CatalogTool): number {
  return tool.pinned?.bytes ?? tool.current?.bytes ?? 0;
}

export function skillDocumentBytes(skill: CatalogSkill): number {
  return textBytes(skill.current_document ?? skill.pinned_document ?? "");
}

export function textBytes(text: string): number {
  return new TextEncoder().encode(text).length;
}
