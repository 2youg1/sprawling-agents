// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The per-model generation draft. Values remain text until the city
// parses them; support corrections are the person's statements, not a
// copy of the upstream or preset rung shown beside them.

import { fill, say } from "../../../core/lang";
import type { Lang } from "../../../core/lang";
import { wireCeiling } from "../../../core/commands";
import { ParameterSupport } from "../../../wire";
import type {
  EndpointSummary,
  GenerationSupport,
  ModelGeneration,
  ModelFactsSummary,
  ParameterAnswer,
} from "../../../wire";

export interface GenerationDraft {
  temperature: string;
  top_p: string;
  top_k: string;
  output: string;
  support: GenerationSupport;
}

export type GenerationDrafts = Record<string, GenerationDraft>;

function stored(summary: EndpointSummary, model: string): ModelGeneration | undefined {
  return (summary.tuning.generation ?? []).find((row) => row.model === model);
}

export function freshGeneration(summary: EndpointSummary): GenerationDrafts {
  return Object.fromEntries(
    summary.models.map((model) => {
      const kept = stored(summary, model.id);
      return [
        model.id,
        {
          temperature: model.generation.settings.temperature ?? "",
          top_p: model.generation.settings.top_p ?? "",
          top_k: model.generation.settings.top_k ?? "",
          output: model.generation.output_tokens?.toString() ?? "",
          support: kept?.support ?? {},
        } satisfies GenerationDraft,
      ];
    }),
  );
}

function present(draft: GenerationDraft): boolean {
  return (
    draft.temperature.trim() !== "" ||
    draft.top_p.trim() !== "" ||
    draft.top_k.trim() !== "" ||
    draft.output.trim() !== "" ||
    draft.support.temperature != null ||
    draft.support.top_p != null ||
    draft.support.top_k != null
  );
}

function output(text: string) {
  const trimmed = text.trim();
  if (!/^[0-9]+$/.test(trimmed)) return null;
  return wireCeiling(Number.parseInt(trimmed, 10));
}

export function generationFrames(
  drafts: GenerationDrafts,
  ids: readonly string[],
): ModelGeneration[] {
  return ids
    .filter((id) => drafts[id] !== undefined && present(drafts[id]))
    .map((id) => {
      const draft = drafts[id];
      if (draft === undefined) return { model: id, settings: {}, support: {} };
      return {
        model: id,
        settings: {
          temperature: draft.temperature.trim() === "" ? null : draft.temperature.trim(),
          top_p: draft.top_p.trim() === "" ? null : draft.top_p.trim(),
          top_k: draft.top_k.trim() === "" ? null : draft.top_k.trim(),
        },
        output_tokens: output(draft.output),
        support: draft.support,
      };
    })
    .sort((left, right) => left.model.localeCompare(right.model));
}

export function sameGeneration(
  left: readonly ModelGeneration[],
  right: readonly ModelGeneration[] | undefined,
): boolean {
  const canonical = (rows: readonly ModelGeneration[] | undefined) =>
    (rows ?? [])
      .map((row) => ({
        model: row.model,
        settings: {
          temperature: row.settings?.temperature ?? null,
          top_p: row.settings?.top_p ?? null,
          top_k: row.settings?.top_k ?? null,
        },
        output_tokens: row.output_tokens ?? null,
        support: {
          temperature: row.support?.temperature ?? null,
          top_p: row.support?.top_p ?? null,
          top_k: row.support?.top_k ?? null,
        },
      }))
      .sort((a, b) => a.model.localeCompare(b.model));
  return JSON.stringify(canonical(left)) === JSON.stringify(canonical(right));
}

const NUMBER = /^-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?$/;
const INTEGER = /^(0|[1-9][0-9]*)$/;

function numeric(text: string): boolean {
  const trimmed = text.trim();
  return trimmed === "" || (NUMBER.test(trimmed) && Number.isFinite(Number(trimmed)));
}

export function generationProblem(
  model: ModelFactsSummary,
  draft: GenerationDraft,
  lang: Lang,
): string | undefined {
  const invalid = (field: string, integer = false) =>
    fill(say(lang, integer ? "generation_integer" : "generation_number"), { field });
  if (!numeric(draft.temperature)) return invalid("temperature");
  if (!numeric(draft.top_p)) return invalid("top_p");
  if (!numeric(draft.top_k) || (!INTEGER.test(draft.top_k.trim()) && draft.top_k.trim() !== ""))
    return invalid("top_k", true);
  if (draft.output.trim() !== "") {
    if (!INTEGER.test(draft.output.trim()) || output(draft.output) === null)
      return say(lang, "generation_positive_integer");
    const wanted = output(draft.output);
    if (wanted !== null && model.max_output_tokens != null && wanted > model.max_output_tokens)
      return fill(say(lang, "generation_output_too_high"), {
        wanted: wanted.toString(),
        ceiling: model.max_output_tokens.toString(),
      });
  }
  for (const [field, answer, value] of [
    ["temperature", model.generation.temperature, draft.temperature],
    ["top_p", model.generation.top_p, draft.top_p],
    ["top_k", model.generation.top_k, draft.top_k],
  ] as const) {
    if ((draft.support[field] ?? answer.support) === "unsupported" && value.trim() !== "")
      return fill(say(lang, "generation_unsupported_value"), { field });
  }
  return undefined;
}

export function supportWord(answer: ParameterAnswer, lang: Lang): string {
  const support = say(lang, `generation_${answer.support}`);
  if (answer.source == null) return support;
  return `${support} · ${say(lang, `generation_from_${answer.source}`)}`;
}

export function generationConflicts(
  model: ModelFactsSummary,
  summary: EndpointSummary,
  draft: GenerationDraft,
): string[] {
  const pointers = new Set(summary.tuning.overrides.map((row) => row.pointer));
  const conflicts: string[] = [];
  for (const [pointer, value] of [
    ["/temperature", draft.temperature],
    ["/top_p", draft.top_p],
    ["/top_k", draft.top_k],
    [model.generation.output_pointer, draft.output],
  ] as const) {
    if (pointers.has(pointer) && value.trim() !== "") conflicts.push(pointer);
  }
  return conflicts;
}

export function supportValue(word: string): ParameterSupport | null {
  return ParameterSupport.members.find((member) => member.literal === word)?.literal ?? null;
}
