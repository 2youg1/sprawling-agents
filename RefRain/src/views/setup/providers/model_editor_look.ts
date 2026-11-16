// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { fill, say } from "../../../core/lang";
import type { Key, Lang } from "../../../core/lang";
import { refusalOf } from "../../../core/probed";
import type { EndpointsAnswer } from "../../../wire";
import type { ButtonProps } from "../../parts/button";
import { roleKey } from "../model_table";
import { generationProblem } from "./generation";
import { refresh, save, savePlan, summaryOf } from "./model_editor";
import type { Hands, ModelEditor, SavePlan } from "./model_editor";

export interface AlertLook {
  readonly title: string;
  readonly subject: string;
  readonly code: string | undefined;
}

export interface ModelEditorLook {
  readonly summary: string;
  readonly generationHeading: string;
  readonly busy: string | undefined;
  readonly removal: string | undefined;
  readonly refresh: ButtonProps;
  readonly save: ButtonProps;
  readonly status: string | undefined;
  readonly alert: AlertLook | undefined;
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
