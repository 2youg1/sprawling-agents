// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the building page's subagent pane reads out of RULES.toml before
// it draws a provider, model and effort. The city is the writer; this
// only finds the tables the file already spells.

import type { Effort } from "../../wire";

export type Tier = "easy" | "medium" | "hard";

export const TIERS: readonly Tier[] = ["easy", "medium", "hard"];

export interface Picked {
  readonly endpoint: string;
  readonly model: string;
  readonly effort: Effort;
}

const EFFORTS: readonly Effort[] = ["minimal", "low", "medium", "high", "xhigh", "max"];

export function choiceOf(text: string): Picked | undefined {
  return tableOf(text, "[workspace_models.choice]");
}

export function tiersOf(text: string): Partial<Record<Tier, Picked>> {
  const found: Partial<Record<Tier, Picked>> = {};
  for (const tier of TIERS) {
    const picked = tableOf(text, `[workspace_models.recommendations.${tier}]`);
    if (picked !== undefined) found[tier] = picked;
  }
  return found;
}

function tableOf(text: string, header: string): Picked | undefined {
  const lines = text.split("\n");
  const start = lines.findIndex((line) => line.trim() === header);
  if (start < 0) return undefined;
  const body: string[] = [];
  for (const line of lines.slice(start + 1)) {
    if (line.trim().startsWith("[")) break;
    body.push(line);
  }
  const endpoint = quoted(body, "endpoint");
  const model = quoted(body, "model");
  const effort = quoted(body, "effort");
  if (endpoint === undefined || model === undefined || !isEffort(effort)) return undefined;
  return { endpoint, model, effort };
}

function quoted(lines: readonly string[], key: string): string | undefined {
  const line = lines.find(
    (each) => each.trim().startsWith(`${key} `) || each.trim().startsWith(`${key}=`),
  );
  if (line === undefined) return undefined;
  const match = /"([^"]*)"/.exec(line);
  return match?.[1];
}

function isEffort(value: string | undefined): value is Effort {
  return EFFORTS.some((effort) => effort === value);
}
