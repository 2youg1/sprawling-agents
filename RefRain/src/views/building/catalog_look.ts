// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Key, Lang } from "../../core/lang";
import { fill, say } from "../../core/lang";
import { count } from "../../core/time";
import { toFragment } from "../../core/route";
import type { Address, CatalogAnswer, Exposure } from "../../wire";
import type { Choice } from "../parts/segmented";
import type { Row, SkillRow, Selection, Scope } from "./catalog";
import {
  EXPOSURE_WORD,
  EXPOSURES,
  effectiveOf,
  effectiveSkill,
  meterOf,
  pressGroup,
  pressRow,
  pressSkill,
  rowsOf,
  selectionOf,
  skillMeterOf,
  skillRowsOf,
} from "./catalog";
import { skillLook, toolLook } from "./catalog_rows";

export type CatalogPane = "tools" | "skills" | "mcp" | "models" | "context" | "rules";

export const CATALOG_PANES: readonly CatalogPane[] = [
  "tools",
  "skills",
  "mcp",
  "models",
  "context",
  "rules",
];

export interface CatalogLook {
  readonly scopeLabel: string;
  readonly scopes: readonly Choice<Scope>[];
  readonly scope: Scope;
  readonly onScope: (scope: Scope) => void;

  readonly panesLabel: string;
  readonly panes: readonly Choice<CatalogPane>[];
  readonly pane: CatalogPane;
  readonly onPane: (pane: CatalogPane) => void;
  readonly meter: string;
  readonly delta: string | undefined;

  readonly beforeRun: string | undefined;
  readonly empty: string | undefined;

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
    context: "catalog_door_context",
    rules: "catalog_door_rules",
  };

  const scoped = seat.pane !== "models" && seat.pane !== "context" && seat.pane !== "rules";
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
        : seat.pane === "context"
          ? say(lang, "catalog_context_note")
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
          case "context":
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
  if (seat.pane === "models" || seat.pane === "context" || seat.pane === "rules") return undefined;

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
