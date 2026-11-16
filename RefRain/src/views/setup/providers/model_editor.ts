// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The state and wiring of an attached endpoint's model disclosure. The
// existing model table owns row presentation; this module owns the
// refresh → edit → save round trip, its shared flight, and the answer
// that proves the save landed. Nothing here touches the DOM or a rune.

import { probeAttached, saveModels, selectModel } from "../../../core/commands";
import type { Admission } from "../../../core/commands";
import { fill, say } from "../../../core/lang";
import type { Key, Lang } from "../../../core/lang";
import { refusalOf } from "../../../core/probed";
import type { ModelFact, Probed } from "../../../core/probed";
import type {
  AxError,
  Command,
  EndpointSummary,
  EndpointsAnswer,
  InputKinds,
  ModelTag,
} from "../../../wire";
import type { BodyOverride, ModelGeneration } from "../../../wire";
import type { ButtonProps } from "../../parts/button";
import { chosenRows, freshTable, setTick } from "../model_rows";
import type { TableState } from "../model_rows";
import { roleKey } from "../model_table";
import type { ModelRow } from "../models";
import { begin, end } from "./flight";
import type { Flight, FlightState } from "./flight";
import { freshGeneration, generationFrames, generationProblem, sameGeneration } from "./generation";
import type { GenerationDrafts } from "./generation";

export interface Hands {
  readonly answer: () => EndpointsAnswer;
  readonly probed: () => Probed | null;
  readonly send: (command: Command) => boolean;
}

interface RolePick {
  readonly id: string;
  readonly tag: ModelTag;
  readonly stated: {
    readonly contextTokens: number | null;
    readonly maxOutputTokens: number | null;
    readonly input: InputKinds | null;
  };
}

interface Expectation {
  readonly models: readonly string[];
  readonly roles: readonly RolePick[];
  readonly generation: readonly ModelGeneration[];
  readonly overrides: readonly BodyOverride[];
}

export interface ModelEditor {
  readonly name: string;
  readonly flight: FlightState;
  readonly table: TableState;
  served: readonly ModelFact[];
  chosen: readonly ModelRow[];
  accepted: readonly RolePick[];
  generation: GenerationDrafts;
  overrides: BodyOverride[];
  probing: { readonly before: Probed | null; readonly operation: Flight } | null;
  saving: {
    readonly against: string;
    readonly operation: Flight;
    readonly expected: Expectation;
  } | null;
  report: AxError | null;
  probeFailure: Probed["failure"];
  saved: readonly string[] | null;
  notSent: boolean;
  focusSummary: boolean;
}

export interface SavePlan {
  readonly admission: Admission;
  readonly expected: Expectation;
  readonly dirty: boolean;
  readonly generation: readonly ModelGeneration[];
  readonly overrides: readonly BodyOverride[];
  readonly invalid: string | null;
  readonly empty: boolean;
  readonly missingRole: ModelTag | null;
}

export interface AlertLook {
  readonly title: string;
  readonly subject: string;
  readonly code: string | undefined;
}

export interface ModelEditorLook {
  readonly summary: string;
  // Always drawn under the disclosure: sampling defaults for the chosen models.
  readonly generationHeading: string;
  readonly busy: string | undefined;
  readonly removal: string | undefined;
  readonly refresh: ButtonProps;
  readonly save: ButtonProps;
  readonly status: string | undefined;
  readonly alert: AlertLook | undefined;
}

function summaryOf(answer: EndpointsAnswer, name: string): EndpointSummary | undefined {
  return answer.endpoints.find((endpoint) => endpoint.name === name);
}

function factsOf(summary: EndpointSummary): readonly ModelFact[] {
  return summary.models
    .filter((model) => model.listing === "listed")
    .map((model) => ({
      id: model.id,
      contextTokens: model.context_tokens ?? null,
      maxOutputTokens: model.max_output_tokens ?? null,
      inputModalities: model.input_modalities,
      inputPrice: model.input_price ?? null,
      outputPrice: model.output_price ?? null,
    }));
}

function rolesOf(answer: EndpointsAnswer, name: string): Record<string, ModelTag> {
  return Object.fromEntries(
    answer.chosen
      .filter((chosen) => chosen.endpoint === name)
      .map((chosen) => [chosen.model, chosen.tag]),
  );
}

function sameIds(left: readonly string[], right: readonly string[]): boolean {
  return [...left].sort().join("\u0000") === [...right].sort().join("\u0000");
}

export function freshModelEditor(
  answer: EndpointsAnswer,
  name: string,
  flight: FlightState,
): ModelEditor {
  const summary = summaryOf(answer, name);
  const served = summary === undefined ? [] : factsOf(summary);
  const table = freshTable({
    ticked: summary?.models.map((model) => model.id) ?? [],
    named:
      summary?.models.filter((model) => model.listing === "named").map((model) => model.id) ?? [],
    roles: rolesOf(answer, name),
  });
  const chosen = chosenRows(table, served);
  return {
    name,
    flight,
    table,
    served,
    chosen,
    accepted: [...draftRoles(chosen).values()],
    generation: summary === undefined ? {} : freshGeneration(summary),
    overrides: summary?.tuning.overrides.map((row) => ({ ...row })) ?? [],
    probing: null,
    saving: null,
    report: null,
    probeFailure: null,
    saved: null,
    notSent: false,
    focusSummary: false,
  };
}

export function choose(editor: ModelEditor, rows: readonly ModelRow[]): void {
  editor.chosen = rows;
  editor.saved = null;
  editor.notSent = false;
  editor.report = null;
}

export function awaiting(editor: ModelEditor): boolean {
  return editor.probing !== null || editor.saving !== null;
}

function sameStated(left: RolePick["stated"], right: RolePick["stated"]): boolean {
  return (
    left.contextTokens === right.contextTokens &&
    left.maxOutputTokens === right.maxOutputTokens &&
    left.input === right.input
  );
}

function sameRole(left: RolePick, right: RolePick): boolean {
  return left.id === right.id && left.tag === right.tag && sameStated(left.stated, right.stated);
}

function draftRoles(rows: readonly ModelRow[]): Map<ModelTag, RolePick> {
  const roles = new Map<ModelTag, RolePick>();
  for (const row of rows) {
    if (row.tag !== null) roles.set(row.tag, { id: row.id, tag: row.tag, stated: row.stated });
  }
  return roles;
}

export function savePlan(editor: ModelEditor, answer: EndpointsAnswer): SavePlan {
  const current = new Map<ModelTag, string>();
  for (const chosen of answer.chosen) {
    if (chosen.endpoint === editor.name) current.set(chosen.tag, chosen.model);
  }
  const draft = draftRoles(editor.chosen);
  const tags = new Set<ModelTag>([...current.keys(), ...draft.keys()]);
  const roles: RolePick[] = [];
  let missingRole: ModelTag | null = null;
  for (const tag of tags) {
    const picked = draft.get(tag);
    if (picked === undefined) {
      if (current.has(tag)) missingRole = tag;
    } else {
      const accepted = editor.accepted.find((role) => role.tag === tag);
      if (
        current.get(tag) !== picked.id ||
        accepted === undefined ||
        !sameStated(accepted.stated, picked.stated)
      )
        roles.push(picked);
    }
  }
  const summary = summaryOf(answer, editor.name);
  const models = editor.chosen.map((row) => row.id);
  const generation = generationFrames(editor.generation, models);
  const overrides = normalizedOverrides(editor.overrides);
  const invalid =
    summary?.models.find((model) => {
      const draft = editor.generation[model.id];
      return (
        models.includes(model.id) &&
        draft !== undefined &&
        generationProblem(model, draft, "en") !== undefined
      );
    })?.id ?? null;
  const listed = new Set(editor.served.map((row) => row.id));
  const changedIds =
    summary === undefined ||
    !sameIds(
      summary.models.map((model) => model.id),
      models,
    );
  return {
    admission: {
      listed: models.filter((id) => listed.has(id)),
      named: models.filter((id) => !listed.has(id)),
    },
    expected: { models, roles, generation, overrides },
    generation,
    overrides,
    invalid,
    dirty:
      changedIds ||
      roles.length > 0 ||
      !sameGeneration(generation, summary.tuning.generation) ||
      !sameOverrides(overrides, summary.tuning.overrides),
    empty: models.length === 0,
    missingRole,
  };
}

function normalizedOverrides(rows: readonly BodyOverride[]): BodyOverride[] {
  return rows
    .filter((row) => row.pointer.trim().startsWith("/"))
    .map((row) => ({ pointer: row.pointer.trim(), value: row.value }));
}

function sameOverrides(left: readonly BodyOverride[], right: readonly BodyOverride[]): boolean {
  return JSON.stringify(normalizedOverrides(left)) === JSON.stringify(normalizedOverrides(right));
}

function unsent(editor: ModelEditor, operation: Flight): void {
  end(editor.flight, operation);
  editor.notSent = true;
  editor.saved = null;
}

export function refresh(editor: ModelEditor, hands: Hands): void {
  const summary = summaryOf(hands.answer(), editor.name);
  if (summary === undefined) return;
  const operation = begin(editor.flight, "probe");
  if (operation === null) return;
  editor.report = null;
  editor.probeFailure = null;
  editor.saved = null;
  editor.notSent = false;
  if (!hands.send(probeAttached(summary))) {
    unsent(editor, operation);
    return;
  }
  editor.probing = { before: hands.probed(), operation };
}

export function save(editor: ModelEditor, hands: Hands): void {
  const answer = hands.answer();
  const summary = summaryOf(answer, editor.name);
  if (summary === undefined) return;
  const plan = savePlan(editor, answer);
  if (!plan.dirty || plan.empty || plan.missingRole !== null || plan.invalid !== null) return;
  const operation = begin(editor.flight, "models");
  if (operation === null) return;
  editor.report = null;
  editor.probeFailure = null;
  editor.saved = null;
  editor.notSent = false;
  const frames = [
    saveModels(summary, plan.admission, plan.generation, plan.overrides),
    ...plan.expected.roles.map((role) => selectModel(summary.name, role.id, role.tag, role.stated)),
  ];
  if (!frames.every((command) => hands.send(command))) {
    unsent(editor, operation);
    return;
  }
  editor.saving = { against: JSON.stringify(summary), operation, expected: plan.expected };
}

function manualIds(manual: string): readonly string[] {
  return manual
    .split(",")
    .map((id) => id.trim())
    .filter((id) => id !== "");
}

export function settleProbe(editor: ModelEditor, found: Probed | null): void {
  const pending = editor.probing;
  if (pending === null || found === null || found === pending.before || found.name !== editor.name)
    return;
  end(editor.flight, pending.operation);
  editor.probing = null;
  editor.probeFailure = found.failure;
  if (found.failure !== null) return;
  const listed = new Set(found.facts.map((row) => row.id));
  const retained = Object.keys(editor.table.ticked).filter(
    (id) => editor.table.ticked[id] === true && !listed.has(id),
  );
  editor.table.manual = [...new Set([...manualIds(editor.table.manual), ...retained])].join(", ");
  editor.served = found.facts;
  editor.chosen = chosenRows(editor.table, editor.served);
}

export function settleAnswer(editor: ModelEditor, answer: EndpointsAnswer): void {
  const pending = editor.saving;
  const summary = summaryOf(answer, editor.name);
  if (
    pending === null ||
    summary === undefined ||
    JSON.stringify(summary) === pending.against ||
    !sameIds(
      summary.models.map((model) => model.id),
      pending.expected.models,
    ) ||
    !pending.expected.roles.every((role) =>
      answer.chosen.some(
        (chosen) =>
          chosen.endpoint === editor.name && chosen.tag === role.tag && chosen.model === role.id,
      ),
    ) ||
    !sameGeneration(summary.tuning.generation ?? [], pending.expected.generation) ||
    !sameOverrides(summary.tuning.overrides, pending.expected.overrides)
  )
    return;
  end(editor.flight, pending.operation);
  editor.saving = null;
  editor.saved = pending.expected.models;
  const current = draftRoles(editor.chosen);
  const settled = pending.expected.roles.filter((role) => {
    const now = current.get(role.tag);
    return now !== undefined && sameRole(now, role);
  });
  editor.accepted = [
    ...editor.accepted.filter(
      (role) => !pending.expected.roles.some((settledRole) => settledRole.tag === role.tag),
    ),
    ...settled,
  ];
  if (
    sameGeneration(
      generationFrames(
        editor.generation,
        editor.chosen.map((row) => row.id),
      ),
      pending.expected.generation,
    )
  )
    editor.generation = freshGeneration(summary);
  if (sameOverrides(editor.overrides, pending.expected.overrides))
    editor.overrides = summary.tuning.overrides.map((row) => ({ ...row }));
  editor.served = factsOf(summary);
  const heldRole = rolesOf(answer, editor.name);
  editor.table.heldRole = heldRole;
  for (const role of settled) editor.table.role[role.id] = heldRole[role.id] ?? role.tag;
  editor.table.refusal = null;
  editor.focusSummary = true;
}

export function refuse(editor: ModelEditor, error: AxError): void {
  if (editor.probing !== null) end(editor.flight, editor.probing.operation);
  if (editor.saving !== null) end(editor.flight, editor.saving.operation);
  editor.probing = null;
  editor.saving = null;
  editor.report = error;
}

export function leave(editor: ModelEditor): void {
  if (editor.probing !== null) end(editor.flight, editor.probing.operation);
  if (editor.saving !== null) end(editor.flight, editor.saving.operation);
  editor.probing = null;
  editor.saving = null;
}

function alertOf(editor: ModelEditor, answer: EndpointsAnswer, lang: Lang): AlertLook | undefined {
  if (editor.notSent)
    return { title: say(lang, "setup_models_not_sent"), subject: "", code: undefined };
  if (editor.report !== null) {
    const host = summaryOf(answer, editor.name)?.base_url ?? editor.name;
    return {
      title:
        refusalOf(editor.report.code) === "reach"
          ? fill(say(lang, "setup_probe_failed"), { host })
          : say(lang, "setup_form_refused"),
      subject: editor.report.subject,
      code: editor.report.code,
    };
  }
  if (editor.probeFailure !== null) {
    const host = summaryOf(answer, editor.name)?.base_url ?? editor.name;
    return {
      title: fill(say(lang, "setup_probe_failed"), { host }),
      subject: editor.probeFailure.subject,
      code: editor.probeFailure.code,
    };
  }
  return undefined;
}

function saveWhy(
  editor: ModelEditor,
  plan: SavePlan,
  answer: EndpointsAnswer,
  lang: Lang,
): string | undefined {
  if (plan.invalid !== null) {
    const model = summaryOf(answer, editor.name)?.models.find((row) => row.id === plan.invalid);
    const draft = editor.generation[plan.invalid];
    if (model !== undefined && draft !== undefined) return generationProblem(model, draft, lang);
  }
  if (plan.empty) return say(lang, "setup_models_empty");
  if (plan.missingRole !== null)
    return fill(say(lang, "setup_models_role_required"), {
      role: say(lang, roleKey(plan.missingRole)),
    });
  return plan.dirty ? undefined : say(lang, "setup_models_unchanged");
}

export function lookOf(editor: ModelEditor, hands: Hands, lang: Lang): ModelEditorLook {
  const answer = hands.answer();
  const summary = summaryOf(answer, editor.name);
  const plan = savePlan(editor, answer);
  const current = editor.flight.current;
  const sharedBusy = current !== null && current.kind === "accounts";
  const removed =
    summary?.models.map((model) => model.id).filter((id) => !plan.expected.models.includes(id)) ??
    [];
  const busy: Key | null =
    editor.probing !== null
      ? "setup_models_probe_busy"
      : editor.saving !== null
        ? "setup_models_save_busy"
        : sharedBusy
          ? "setup_endpoint_busy"
          : null;
  const unavailable =
    current !== null && editor.saving === null
      ? say(lang, "setup_endpoint_busy")
      : saveWhy(editor, plan, answer, lang);
  return {
    summary: fill(say(lang, "setup_models_toggle"), { n: String(summary?.models.length ?? 0) }),
    generationHeading: say(lang, "setup_generation"),
    busy: busy === null ? undefined : say(lang, busy),
    removal:
      removed.length === 0
        ? undefined
        : fill(say(lang, "setup_models_will_remove"), { models: removed.join(", ") }),
    refresh: {
      label: say(lang, "setup_models_refresh"),
      tone: "secondary",
      loading: editor.probing !== null,
      ...(current !== null && editor.probing === null
        ? { why: say(lang, "setup_endpoint_busy") }
        : {}),
      onPress: () => {
        refresh(editor, hands);
      },
    },
    save: {
      label: say(lang, "setup_models_save"),
      tone: "primary",
      loading: editor.saving !== null,
      ...(unavailable === undefined ? {} : { why: unavailable }),
      onPress: () => {
        save(editor, hands);
      },
    },
    status:
      editor.saved === null
        ? undefined
        : fill(say(lang, "setup_models_saved"), { models: editor.saved.join(", ") }),
    alert: alertOf(editor, answer, lang),
  };
}

// Used by wiring tests to drive the same tick path as the table without
// importing a look.
export function untick(editor: ModelEditor, id: string): void {
  setTick(editor.table, id, false);
  editor.chosen = chosenRows(editor.table, editor.served);
}
