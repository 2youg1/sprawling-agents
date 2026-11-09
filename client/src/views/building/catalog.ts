// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The unified building catalog draft. Tool exposure proofs live in
// `client/spec/Views/Catalog.lean`; skill pins, connector references and
// Reading Room names travel beside that table so one save writes one selection.

import type { Key, Lang } from "../../core/lang";
import { fill, say } from "../../core/lang";
import { ago, count } from "../../core/time";
import { toFragment } from "../../core/route";
import type {
  Address,
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
import type { Choice } from "../parts/segmented";

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

// What a refreshed answer does to a save in flight. When the answer
// matches what was sent, the save landed; the draft resets only when
// the person has not edited since sending, so a quick second edit made
// inside the round trip is not clobbered by the acknowledgement.
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

const CHANGE_WORD: Record<CatalogChange, Key> = {
  added: "catalog_added",
  changed: "catalog_changed",
  removed: "catalog_removed",
};

// Which pane of the catalog is drawn: tools the building owns, skills,
// MCP connectors, subagent model recommendations (via RULES.toml), and
// the RULES.toml editor itself.
export type CatalogPane = "tools" | "skills" | "mcp" | "models" | "rules";

export const CATALOG_PANES: readonly CatalogPane[] = ["tools", "skills", "mcp", "models", "rules"];

export interface CatalogLook {
  readonly scopeLabel: string;
  readonly scopes: readonly Choice<Scope>[];
  readonly scope: Scope;
  readonly onScope: (scope: Scope) => void;
  // Own name for the pane radiogroup; never the first pane's label.
  readonly panesLabel: string;
  readonly panes: readonly Choice<CatalogPane>[];
  readonly pane: CatalogPane;
  readonly onPane: (pane: CatalogPane) => void;
  readonly meter: string;
  readonly delta: string | undefined;
  // Said on the tools pane before this building has frozen a run: the
  // rows are the tools that run will admit, and a save reaches it.
  readonly beforeRun: string | undefined;
  readonly empty: string | undefined;
  // Shown on the models and rules panes: what RULES.toml governs here.
  readonly rulesNote: string | undefined;
  readonly groups: readonly GroupLook[];
  readonly save: {
    readonly label: string;
    readonly why: string | undefined;
    readonly onPress: () => void;
  };
  readonly openSessions:
    { readonly text: string; readonly href: string; readonly label: string } | undefined;
  readonly unused: {
    readonly label: string;
    readonly pressed: boolean;
    readonly onPress: () => void;
    readonly offLabel: string;
    readonly onOff: () => void;
  };
  readonly copy:
    | {
        readonly label: string;
        readonly options: readonly Address[];
        readonly onPick: (addr: Address) => void;
      }
    | undefined;
}

export interface GroupLook {
  readonly key: string;
  readonly title: string;
  readonly summary: string;
  readonly open: boolean;
  readonly choices: readonly Choice<Exposure>[];
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
  readonly change: string | undefined;
  readonly comparison:
    | { readonly pinned: string; readonly current: string; readonly audit: string | undefined }
    | undefined;
  readonly actions: readonly {
    readonly key: string;
    readonly label: string;
    readonly why: string | undefined;
    readonly onPress: () => void;
  }[];
}

export interface Seat {
  readonly answer: CatalogAnswer;
  readonly scope: Scope;
  readonly draft: Selection;
  readonly edited: boolean;
  readonly now: number;
  readonly unusedOnly: boolean;
  readonly buildings: readonly Address[];
  readonly pane: CatalogPane;
}

export interface Hands {
  readonly scope: (scope: Scope) => void;
  readonly pane: (pane: CatalogPane) => void;
  readonly edit: (draft: Selection) => void;
  readonly save: () => void;
  readonly unused: (only: boolean) => void;
  readonly copy: (building: Address) => void;
}

const OPEN_GROUP = 12;

export function catalogLook(seat: Seat, lang: Lang, hands: Hands): CatalogLook {
  const allTools = rowsOf(seat.answer, seat.scope);
  const allSkills = skillRowsOf(seat.answer, seat.scope);
  const tools = seat.unusedOnly
    ? allTools.filter((row) => row.tool.runs_used === 0 && row.tool.exposure !== "off")
    : allTools;
  const skills = seat.unusedOnly
    ? allSkills.filter((row) => row.skill.runs_used === 0 && row.skill.exposure !== "off")
    : allSkills;
  const saved = selectionOf(seat.answer);
  const delta =
    meterOf(saved.catalog, seat.draft.catalog, allTools) +
    skillMeterOf(saved, seat.draft, allSkills);
  const choices = (why: string | undefined): readonly Choice<Exposure>[] =>
    EXPOSURES.map((value) => ({
      value,
      label: say(lang, EXPOSURE_WORD[value]),
      ...(why === undefined ? {} : { why }),
    }));
  const mcpLabels: ReadonlySet<string> = new Set(
    seat.draft.references.map((reference) => reference.label),
  );
  const namespaces = [...new Set(tools.map((row) => row.tool.namespace))];
  const toolNamespaces = namespaces.filter((namespace) => !mcpLabels.has(namespace));
  const mcpNamespaces = namespaces.filter((namespace) => mcpLabels.has(namespace));
  const toolGroups = toolNamespaces.map((namespace) => {
    const members = tools.filter((row) => row.tool.namespace === namespace);
    return toolGroup(namespace, members, seat, lang, hands, choices);
  });
  const mcpGroups = mcpNamespaces.map((namespace) => {
    const members = tools.filter((row) => row.tool.namespace === namespace);
    return toolGroup(namespace, members, seat, lang, hands, choices);
  });
  // A connector with no tool row yet still names itself so the pane is
  // not empty while the city has only the reference. Under the unused
  // filter those placeholders are not unused tools, so they stay out.
  if (!seat.unusedOnly) {
    for (const reference of seat.draft.references) {
      if (mcpGroups.some((group) => group.key === reference.label)) continue;
      mcpGroups.push({
        key: reference.label,
        title: reference.label,
        summary: say(lang, "catalog_mcp_reference"),
        open: true,
        choices: choices(undefined),
        held: null,
        onPick: () => undefined,
        rows: [],
      });
    }
  }
  const skillGroups = skills.length > 0 ? [skillGroup(skills, seat, lang, hands, choices)] : [];
  const groups =
    seat.pane === "tools"
      ? toolGroups
      : seat.pane === "skills"
        ? skillGroups
        : seat.pane === "mcp"
          ? mcpGroups
          : [];
  const paneWord: Record<CatalogPane, Key> = {
    tools: "catalog_door_tools",
    skills: "catalog_door_skills",
    mcp: "catalog_door_mcp",
    models: "catalog_door_models",
    rules: "catalog_door_rules",
  };
  // RULES.toml and model notes are always this building's file; the
  // catalog's city/building scope is for the tool table alone.
  const scoped = seat.pane !== "models" && seat.pane !== "rules";
  return {
    scopeLabel: say(lang, "catalog_scope"),
    scopes: scoped
      ? [
          { value: "building", label: say(lang, "catalog_scope_building") },
          { value: "city", label: say(lang, "catalog_scope_city") },
        ]
      : [],
    scope: seat.scope,
    onScope: hands.scope,
    panesLabel: say(lang, "catalog_panes"),
    panes: CATALOG_PANES.map((value) => ({ value, label: say(lang, paneWord[value]) })),
    pane: seat.pane,
    onPane: hands.pane,
    meter: fill(say(lang, "catalog_meter_unified"), {
      always: String(seat.answer.always),
      dormant: String(seat.answer.dormant),
      skills: String(seat.answer.skills),
      core: count(seat.answer.core_bytes),
      index: count(seat.answer.index_bytes),
      reading: count(seat.answer.reading_bytes),
    }),
    delta:
      seat.edited && delta !== 0
        ? fill(say(lang, delta > 0 ? "catalog_more" : "catalog_less"), {
            n: count(Math.abs(delta)),
          })
        : undefined,
    beforeRun:
      seat.pane === "tools" && seat.answer.run == null && groups.length > 0
        ? say(lang, "catalog_before_run")
        : undefined,
    empty: emptyOf(seat, groups.length, lang),
    rulesNote:
      seat.pane === "models"
        ? say(lang, "catalog_models_note")
        : seat.pane === "rules"
          ? say(lang, "catalog_rules_note")
          : undefined,
    groups,
    save: {
      label: say(lang, seat.scope === "building" ? "catalog_save_building" : "catalog_save_city"),
      why: seat.edited ? undefined : say(lang, "catalog_unchanged"),
      onPress: hands.save,
    },
    openSessions:
      seat.answer.open_sessions > 0
        ? {
            text: fill(say(lang, "catalog_open_sessions"), {
              n: String(seat.answer.open_sessions),
            }),
            href: toFragment({ kind: "talk", address: seat.answer.building }),
            label: "/new",
          }
        : undefined,
    unused: {
      label: say(lang, "catalog_unused_filter"),
      pressed: seat.unusedOnly,
      onPress: () => {
        hands.unused(!seat.unusedOnly);
      },
      offLabel: say(lang, "catalog_turn_off_unused"),
      onOff: () => {
        // Only the pane on screen: turning off unused MCP tools must not
        // also clear unused skills the person cannot see.
        let next = seat.draft;
        switch (seat.pane) {
          case "tools":
            for (const row of tools.filter((each) => !mcpLabels.has(each.tool.namespace))) {
              next = { ...next, catalog: pressRow(next.catalog, row, "off") };
            }
            break;
          case "mcp":
            for (const row of tools.filter((each) => mcpLabels.has(each.tool.namespace))) {
              next = { ...next, catalog: pressRow(next.catalog, row, "off") };
            }
            break;
          case "skills":
            for (const row of skills) next = pressSkill(next, row, "off", seat.scope);
            break;
          case "models":
          case "rules":
            break;
        }
        hands.edit(next);
      },
    },
    copy:
      seat.scope === "building"
        ? {
            label: say(lang, "catalog_copy_selection"),
            options: seat.buildings.filter((addr) => addr !== seat.answer.building),
            onPick: hands.copy,
          }
        : undefined,
  };
}

function emptyOf(seat: Seat, groups: number, lang: Lang): string | undefined {
  if (seat.pane === "models" || seat.pane === "rules") return undefined;
  // Only the tools pane says a run is what fills it. Skills and connectors
  // have their own empty sentences, and a tools pane that already lists
  // the standing tools is not empty.
  if (seat.pane === "tools" && seat.answer.run == null && groups === 0) {
    return say(lang, "catalog_no_run");
  }
  if (groups === 0 && seat.unusedOnly) return say(lang, "catalog_no_unused");
  if (groups === 0) {
    return say(lang, seat.pane === "mcp" ? "catalog_mcp_empty" : "catalog_pane_empty");
  }
  return undefined;
}

function toolGroup(
  namespace: string,
  members: readonly Row[],
  seat: Seat,
  lang: Lang,
  hands: Hands,
  choices: (why: string | undefined) => readonly Choice<Exposure>[],
): GroupLook {
  const held = members.map((row) => effectiveOf(seat.draft.catalog, row));
  return {
    key: namespace,
    title: namespace,
    summary: groupSummary(members.length, held, lang),
    open: members.length <= OPEN_GROUP,
    choices: choices(members.every((row) => row.fixed) ? say(lang, "catalog_fixed") : undefined),
    held: held.every((each) => each === held[0]) ? (held[0] ?? null) : null,
    onPick: (value) => {
      hands.edit({
        ...seat.draft,
        catalog: pressGroup(members, seat.draft.catalog, namespace, value),
      });
    },
    rows: members.map((row) => toolLook(row, seat, lang, hands, choices)),
  };
}

function skillGroup(
  members: readonly SkillRow[],
  seat: Seat,
  lang: Lang,
  hands: Hands,
  choices: (why: string | undefined) => readonly Choice<Exposure>[],
): GroupLook {
  const held = members.map((row) =>
    effectiveSkill(seat.draft.catalog, row, seat.draft.readingRoom.includes(row.skill.name)),
  );
  return {
    key: "skill",
    title: say(lang, "bld_skills"),
    summary: groupSummary(members.length, held, lang),
    open: members.length <= OPEN_GROUP,
    choices: choices(undefined),
    held: held.every((each) => each === held[0]) ? (held[0] ?? null) : null,
    onPick: (value) => {
      let next = seat.draft;
      for (const row of members) next = pressSkill(next, row, value, seat.scope);
      hands.edit(next);
    },
    rows: members.map((row) => skillLook(row, seat, lang, hands, choices)),
  };
}

function groupSummary(n: number, held: readonly Exposure[], lang: Lang): string {
  const tally = EXPOSURES.map((value) => held.filter((each) => each === value).length);
  return fill(say(lang, "catalog_group"), {
    n: String(n),
    core: String(tally[0] ?? 0),
    dormant: String(tally[1] ?? 0),
    off: String(tally[2] ?? 0),
  });
}

function toolLook(
  row: Row,
  seat: Seat,
  lang: Lang,
  hands: Hands,
  choices: (why: string | undefined) => readonly Choice<Exposure>[],
): RowLook {
  const tool = row.tool;
  const current = tool.current ?? tool.pinned;
  const actions =
    seat.scope === "building" && tool.change != null
      ? [
          {
            key: "accept",
            label: say(lang, "catalog_accept"),
            why: selectionPinsTool(seat.draft, tool) ? say(lang, "catalog_accepted") : undefined,
            onPress: () => {
              hands.edit(acceptTool(seat.draft, tool));
            },
          },
        ]
      : [];
  return {
    key: tool.name,
    name: tool.name,
    hint: current?.hint ?? "",
    label: fill(say(lang, "catalog_row"), { name: tool.name }),
    choices: choices(row.fixed ? say(lang, "catalog_fixed") : undefined),
    held: effectiveOf(seat.draft.catalog, row),
    onPick: (value) => {
      hands.edit({ ...seat.draft, catalog: pressRow(seat.draft.catalog, row, value) });
    },
    facts: factsOf(
      tool.from,
      tool.runs_used,
      tool.last_used,
      seat,
      lang,
      current?.bytes ?? 0,
      current?.hint ?? "",
    ),
    change: tool.change == null ? undefined : say(lang, CHANGE_WORD[tool.change]),
    comparison: comparisonOf(
      tool.pinned?.description,
      tool.current?.description,
      tool.pinned?.input_schema,
      tool.current?.input_schema,
      undefined,
    ),
    actions,
  };
}

function skillLook(
  row: SkillRow,
  seat: Seat,
  lang: Lang,
  hands: Hands,
  choices: (why: string | undefined) => readonly Choice<Exposure>[],
): RowLook {
  const skill = row.skill;
  const accepted = selectionPinsSkill(seat.draft, skill);
  const kept = skill.pinned != null && seat.draft.skillPins[skill.name] === skill.pinned;
  const actions =
    seat.scope === "building" && skill.change != null
      ? [
          {
            key: "accept",
            label: say(lang, "catalog_accept"),
            why: accepted ? say(lang, "catalog_accepted") : undefined,
            onPress: () => {
              hands.edit(acceptSkill(seat.draft, skill));
            },
          },
          ...(skill.pinned == null
            ? []
            : [
                {
                  key: "keep",
                  label: say(lang, "catalog_keep"),
                  why: kept ? say(lang, "catalog_kept") : undefined,
                  onPress: () => {
                    hands.edit(keepSkill(seat.draft, skill));
                  },
                },
              ]),
        ]
      : [];
  const audit =
    skill.audit == null
      ? undefined
      : fill(say(lang, skill.audit_stale ? "catalog_audit_stale" : "catalog_audit"), {
          verdict: skill.audit,
        });
  return {
    key: `skill:${skill.name}`,
    name: skill.name,
    hint: skill.disclosure,
    label: fill(say(lang, "catalog_row"), { name: skill.name }),
    choices: choices(undefined),
    held: effectiveSkill(seat.draft.catalog, row, seat.draft.readingRoom.includes(skill.name)),
    onPick: (value) => {
      hands.edit(pressSkill(seat.draft, row, value, seat.scope));
    },
    facts: factsOf(
      skill.from,
      skill.runs_used,
      skill.last_used,
      seat,
      lang,
      skillDocumentBytes(skill),
      skill.disclosure,
    ),
    change: skill.change == null ? undefined : say(lang, CHANGE_WORD[skill.change]),
    comparison:
      skill.change == null
        ? undefined
        : {
            pinned: skill.pinned_document ?? "",
            current: skill.current_document ?? "",
            audit,
          },
    actions,
  };
}

function factsOf(
  from: ExposureFrom,
  runs: number,
  last: number | null | undefined,
  seat: Seat,
  lang: Lang,
  coreBytes: number,
  hint: string,
): string {
  const used =
    runs > 0 && last != null
      ? fill(say(lang, "catalog_used"), {
          n: String(runs),
          of: String(seat.answer.runs_seen),
          when: ago(lang, last, seat.now),
        })
      : fill(say(lang, "catalog_unused"), { of: String(seat.answer.runs_seen) });
  return [
    say(lang, FROM_WORD[from]),
    used,
    fill(say(lang, "catalog_costs"), { core: count(coreBytes), dormant: count(textBytes(hint)) }),
  ].join(" · ");
}

function comparisonOf(
  pinnedText: string | undefined,
  currentText: string | undefined,
  pinnedSchema: Readonly<Record<string, unknown>> | undefined,
  currentSchema: Readonly<Record<string, unknown>> | undefined,
  audit: string | undefined,
): RowLook["comparison"] {
  if (pinnedText === undefined && currentText === undefined) return undefined;
  return {
    pinned: `${pinnedText ?? ""}\n${JSON.stringify(pinnedSchema ?? {}, null, 2)}`,
    current: `${currentText ?? ""}\n${JSON.stringify(currentSchema ?? {}, null, 2)}`,
    audit,
  };
}

function selectionPinsTool(selection: Selection, tool: CatalogTool): boolean {
  const current = tool.current;
  return selection.references.some((reference) => {
    if (reference.label !== tool.namespace || reference.tools == null) return false;
    const held = reference.tools[tool.name];
    return current == null ? held === undefined : held === current.digest;
  });
}

function selectionPinsSkill(selection: Selection, skill: CatalogSkill): boolean {
  const held = selection.skillPins[skill.name];
  return skill.candidate == null ? held === undefined : held === skill.candidate;
}

function bytesOf(tool: CatalogTool): number {
  return tool.current?.bytes ?? tool.pinned?.bytes ?? 0;
}

function savedBytesOf(tool: CatalogTool): number {
  return tool.pinned?.bytes ?? tool.current?.bytes ?? 0;
}

function skillDocumentBytes(skill: CatalogSkill): number {
  return textBytes(skill.current_document ?? skill.pinned_document ?? "");
}

function textBytes(text: string): number {
  return new TextEncoder().encode(text).length;
}
