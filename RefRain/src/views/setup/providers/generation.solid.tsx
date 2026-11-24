// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createSignal, For, Show } from "solid-js";
import { fill, say, type Lang } from "../../../core/lang";
import { storeValue } from "../../../core/solid_store";
import { ui } from "../../../ui";
import type { BodyOverride, EndpointSummary, ModelFactsSummary } from "../../../wire";
import Button from "../../parts/button.solid";
import Field from "../../parts/field.solid";
import {
  generationConflicts,
  generationProblem,
  supportValue,
  supportWord,
  type GenerationDraft,
  type GenerationDrafts,
} from "./generation";

export interface GenerationProps {
  readonly models: readonly ModelFactsSummary[];
  readonly summary: EndpointSummary;
  readonly drafts: GenerationDrafts;
  readonly overrideRows: BodyOverride[];
  readonly onEdit: () => void;
  readonly open?: boolean | undefined;
}

const FIELDS = ["temperature", "top_p", "top_k"] as const;

function ModelFields(props: {
  readonly lang: Lang;
  readonly model: ModelFactsSummary;
  readonly summary: EndpointSummary;
  readonly draft: GenerationDraft;
  readonly open: boolean;
  readonly onEdit: () => void;
  readonly revision: number;
}) {
  const draft = () => {
    void props.revision;
    return props.draft;
  };
  const problem = () => generationProblem(props.model, draft(), props.lang);
  const conflicts = () => generationConflicts(props.model, props.summary, draft());
  const change = (field: "temperature" | "top_p" | "top_k" | "output", value: string) => {
    draft()[field] = value;
    props.onEdit();
  };
  return (
    <details class="rounded-card border border-edge-panel px-snug py-tight" open={props.open}>
      <summary data-control="" class="cursor-pointer font-mono text-note text-text-quiet">
        {fill(say(props.lang, "generation_toggle"), { model: props.model.id })}
      </summary>
      <div class="grid grid-cols-[repeat(auto-fit,minmax(320px,1fr))] gap-snug pt-snug">
        <For each={FIELDS}>
          {(field) => {
            const fallback = () => props.model.generation[field];
            const support = () => draft().support[field] ?? fallback().support;
            return (
              <div class="flex flex-col gap-tight">
                <label class="flex flex-col gap-tight text-note text-text-quiet">
                  {fill(say(props.lang, "generation_support"), { field })}
                  <select
                    class="h-control rounded-control bg-raised px-tight text-text"
                    value={support()}
                    onChange={(event) => {
                      const asked = supportValue(event.currentTarget.value);
                      if (asked === null) return;
                      const current = draft();
                      current.support = { ...current.support, [field]: asked };
                      props.onEdit();
                    }}
                  >
                    <option value="unknown">{say(props.lang, "generation_unknown")}</option>
                    <option value="supported">{say(props.lang, "generation_supported")}</option>
                    <option value="unsupported">{say(props.lang, "generation_unsupported")}</option>
                  </select>
                </label>
                <p class="text-note text-text-faint">{supportWord(fallback(), props.lang)}</p>
                <Show
                  when={support() !== "unsupported"}
                  fallback={
                    <Show when={draft()[field].trim() !== ""}>
                      <Button
                        label={fill(say(props.lang, "generation_clear_value"), { field })}
                        tone="quiet"
                        onPress={() => change(field, "")}
                      />
                    </Show>
                  }
                >
                  <Field
                    label={field}
                    mono
                    value={draft()[field]}
                    onInput={(value) => change(field, value)}
                  />
                </Show>
              </div>
            );
          }}
        </For>
        <Field
          label={say(props.lang, "generation_output")}
          kind="number"
          step={1024}
          mono
          value={draft().output}
          onInput={(value) => change("output", value)}
        />
      </div>
      <For each={conflicts()}>
        {(pointer) => (
          <p class="pt-tight font-mono text-note text-alert">
            {fill(say(props.lang, "generation_override"), { pointer })}
          </p>
        )}
      </For>
      <Show when={problem()}>
        {(why) => (
          <p role="alert" class="pt-tight text-note text-alert">
            {why()}
          </p>
        )}
      </Show>
    </details>
  );
}

export default function Generation(props: GenerationProps) {
  const lang = storeValue(ui().lang);
  const [revision, setRevision] = createSignal(0);
  function edited(): void {
    setRevision((version) => version + 1);
    props.onEdit();
  }
  const overrides = () => {
    revision();
    return [...props.overrideRows];
  };
  function update(at: number, field: "pointer" | "value", value: string): void {
    const row = props.overrideRows[at];
    if (row === undefined) return;
    props.overrideRows[at] = { ...row, [field]: value };
    edited();
  }
  return (
    <div class="flex flex-col gap-tight">
      <p class="text-note text-text-faint">{say(lang(), "generation_new_sessions")}</p>
      <For each={props.models} keyed={(model) => model.id}>
        {(model) => (
          <Show when={props.drafts[model().id]}>
            {(row) => (
              <ModelFields
                model={model()}
                draft={row()}
                summary={props.summary}
                lang={lang()}
                open={props.open ?? false}
                revision={revision()}
                onEdit={edited}
              />
            )}
          </Show>
        )}
      </For>
      <details class="pt-tight">
        <summary data-control="" class="cursor-pointer text-note text-text-quiet">
          {say(lang(), "setup_overrides")}
        </summary>
        <div class="flex flex-col gap-snug pt-snug">
          <For each={overrides()} keyed={false}>
            {(row, at) => (
              <div class="grid grid-cols-[repeat(auto-fit,minmax(320px,1fr))] items-end gap-snug">
                <Field
                  label={say(lang(), "setup_pointer")}
                  mono
                  value={row().pointer}
                  onInput={(value) => update(at, "pointer", value)}
                />
                <Field
                  label={say(lang(), "setup_override_value")}
                  mono
                  value={row().value}
                  onInput={(value) => update(at, "value", value)}
                />
                <Button
                  label={say(lang(), "setup_remove_row")}
                  tone="quiet"
                  onPress={() => {
                    props.overrideRows.splice(at, 1);
                    edited();
                  }}
                />
              </div>
            )}
          </For>
          <span>
            <Button
              label={say(lang(), "setup_add_row")}
              tone="secondary"
              onPress={() => {
                props.overrideRows.push({ pointer: "", value: "" });
                edited();
              }}
            />
          </span>
        </div>
      </details>
    </div>
  );
}
