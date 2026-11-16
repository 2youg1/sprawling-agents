// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Lang } from "../../core/lang";
import { fill, say } from "../../core/lang";
import { ago, count } from "../../core/time";
import type { CatalogSkill, CatalogTool, Exposure, ExposureFrom } from "../../wire";
import type { Choice } from "../parts/segmented";
import type { Row, SkillRow, Selection } from "./catalog";
import {
  CHANGE_WORD,
  FROM_WORD,
  acceptSkill,
  acceptTool,
  effectiveOf,
  effectiveSkill,
  keepSkill,
  pressRow,
  pressSkill,
  skillDocumentBytes,
  textBytes,
} from "./catalog";
import type { Hands, RowLook, Seat } from "./catalog_look";

export function toolLook(
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

export function skillLook(
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
  if (pinnedText === undefined || currentText === undefined) return undefined;
  return {
    pinned: `${pinnedText}\n${JSON.stringify(pinnedSchema ?? {}, null, 2)}`,
    current: `${currentText}\n${JSON.stringify(currentSchema ?? {}, null, 2)}`,
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
