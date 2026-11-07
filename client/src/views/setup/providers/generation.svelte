<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  import { fill, say } from "../../../core/lang";
  import { ui } from "../../../ui";
  import type { BodyOverride, EndpointSummary, ModelFactsSummary } from "../../../wire";
  import Button from "../../parts/button.svelte";
  import Field from "../../parts/field.svelte";
  import { generationConflicts, generationProblem, supportValue, supportWord } from "./generation";
  import type { GenerationDraft, GenerationDrafts } from "./generation";

  interface Props {
    readonly models: readonly ModelFactsSummary[];
    readonly summary: EndpointSummary;
    readonly drafts: GenerationDrafts;
    readonly overrideRows: BodyOverride[];
    readonly onEdit: () => void;
    readonly open?: boolean;
  }
  const { models, summary, drafts, overrideRows, onEdit, open = false }: Props = $props();
  const u = ui();
  const lang = u.lang;

  function input(
    draft: GenerationDraft,
    field: "temperature" | "top_p" | "top_k" | "output",
    value: string,
  ) {
    draft[field] = value;
    onEdit();
  }

  function support(draft: GenerationDraft, field: "temperature" | "top_p" | "top_k", word: string) {
    const picked = supportValue(word);
    if (picked === null) return;
    draft.support = { ...draft.support, [field]: picked };
    onEdit();
  }
</script>

<div class="flex flex-col gap-tight">
  <p class="text-note text-text-faint">{say($lang, "generation_new_sessions")}</p>
  {#each models as model (model.id)}
    {@const draft = drafts[model.id]}
    {#if draft !== undefined}
      {@const problem = generationProblem(model, draft, $lang)}
      <details class="rounded-card border border-edge-panel px-snug py-tight" {open}>
        <summary class="cursor-pointer font-mono text-note text-text-quiet">
          {fill(say($lang, "generation_toggle"), { model: model.id })}
        </summary>
        <div class="grid grid-cols-[repeat(auto-fit,minmax(320px,1fr))] gap-snug pt-snug">
          {#each [{ field: "temperature", answer: model.generation.temperature }, { field: "top_p", answer: model.generation.top_p }, { field: "top_k", answer: model.generation.top_k }] as const as item (item.field)}
            <div class="flex flex-col gap-tight">
              <label class="flex flex-col gap-tight text-note text-text-quiet">
                {fill(say($lang, "generation_support"), { field: item.field })}
                <select
                  class="h-control rounded-control bg-raised px-tight text-text"
                  value={draft.support[item.field] ?? item.answer.support}
                  onchange={(event) => {
                    if (event.currentTarget instanceof HTMLSelectElement)
                      support(draft, item.field, event.currentTarget.value);
                  }}
                >
                  <option value="unknown">{say($lang, "generation_unknown")}</option>
                  <option value="supported">{say($lang, "generation_supported")}</option>
                  <option value="unsupported">{say($lang, "generation_unsupported")}</option>
                </select>
              </label>
              <p class="text-note text-text-faint">{supportWord(item.answer, $lang)}</p>
              {#if (draft.support[item.field] ?? item.answer.support) !== "unsupported"}
                <Field
                  label={item.field}
                  mono
                  value={draft[item.field]}
                  onInput={(value: string) => {
                    input(draft, item.field, value);
                  }}
                />
              {:else if draft[item.field].trim() !== ""}
                <Button
                  label={fill(say($lang, "generation_clear_value"), { field: item.field })}
                  tone="quiet"
                  onPress={() => {
                    draft[item.field] = "";
                    onEdit();
                  }}
                />
              {/if}
            </div>
          {/each}
          <Field
            label={say($lang, "generation_output")}
            kind="number"
            step={1024}
            mono
            value={draft.output}
            onInput={(value: string) => {
              input(draft, "output", value);
            }}
          />
        </div>
        {#each generationConflicts(model, summary, draft) as pointer (pointer)}
          <p class="pt-tight font-mono text-note text-alert">
            {fill(say($lang, "generation_override"), { pointer })}
          </p>
        {/each}
        {#if problem !== undefined}
          <p role="alert" class="pt-tight text-note text-alert">{problem}</p>
        {/if}
      </details>
    {/if}
  {/each}
  <details class="pt-tight">
    <summary class="cursor-pointer text-note text-text-quiet"
      >{say($lang, "setup_overrides")}</summary
    >
    <div class="flex flex-col gap-snug pt-snug">
      {#each overrideRows as row, at (at)}
        <div class="grid grid-cols-[repeat(auto-fit,minmax(320px,1fr))] items-end gap-snug">
          <Field
            label={say($lang, "setup_pointer")}
            mono
            value={row.pointer}
            onInput={(value: string) => {
              overrideRows[at] = { ...row, pointer: value };
              onEdit();
            }}
          />
          <Field
            label={say($lang, "setup_override_value")}
            mono
            value={row.value}
            onInput={(value: string) => {
              overrideRows[at] = { ...row, value };
              onEdit();
            }}
          />
          <Button
            label={say($lang, "setup_remove_row")}
            tone="quiet"
            onPress={() => {
              overrideRows.splice(at, 1);
              onEdit();
            }}
          />
        </div>
      {/each}
      <span>
        <Button
          label={say($lang, "setup_add_row")}
          tone="secondary"
          onPress={() => {
            overrideRows.push({ pointer: "", value: "" });
            onEdit();
          }}
        />
      </span>
    </div>
  </details>
</div>
