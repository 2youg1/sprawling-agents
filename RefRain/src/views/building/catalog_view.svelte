<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts">
  import type { Answered } from "../../core/answered";
  import { QUERIES } from "../../core/asking";
  import { fill, say } from "../../core/lang";
  import { toFragment } from "../../core/route";
  import { kilo } from "../../core/time";
  import { ui } from "../../ui";
  import type { Address, Effort, Query } from "../../wire";
  import { TIERS } from "./subagent";
  import type { Tier } from "./subagent";
  import Button from "../parts/button.svelte";
  import Field from "../parts/field.svelte";
  import Unanswered from "../parts/unanswered.svelte";
  import { rulesAt } from "../settings/files";
  import { edited as markEdited } from "../settings/saving";
  import type { Saving } from "../settings/saving";
  import { textBoxWire } from "./text_box";
  import TextBox from "./text_box.look.svelte";
  import Look from "./catalog.look.svelte";
  import type { CatalogLook, CatalogPane } from "./catalog_look";

  interface Endpoint {
    name: string;
    label: string;
    models: readonly { id: string; thinking: { levels: readonly Effort[] } }[];
  }

  interface Props {
    building: Address;
    read: Answered<unknown>;
    look: CatalogLook | undefined;
    pane: CatalogPane;
    saved: boolean;
    question: Query;
    rulesFailed: { query: string; reason: string | null } | null;
    onDisk: string | null;
    rulesDraft: string;
    rulesSaving: Saving;
    endpointRead: Answered<unknown>;
    endpointList: readonly Endpoint[];
    providerModels: readonly { id: string; thinking: { levels: readonly Effort[] } }[];
    levels: readonly Effort[];
    shortcuts: Partial<Record<Tier, { endpoint: string; model: string; effort: Effort }>>;
    choiceBlocked: string | undefined;
    provider: string;
    modelId: string;
    level: Effort;
    lockBox: string;
    lockEdited: boolean;
    lockField: { kind: "clear" } | { kind: "set"; tokens: number } | { kind: "invalid" };
    modelWindow: number | null;
    share: number | null;
    saveRules: () => void;
    saveLock: () => void;
    saveChoice: () => void;
    useTier: (tier: Tier) => void;
    setRulesDraft: (next: string) => void;
    setRulesSaving: (next: Saving) => void;
    setProvider: (next: string) => void;
    setModelId: (next: string) => void;
    setLevel: (next: Effort) => void;
    setChoiceDirty: (next: boolean) => void;
    setLockBox: (next: string) => void;
    setLockEdited: (next: boolean) => void;
  }

  const {
    building,
    read,
    look,
    pane,
    saved,
    question,
    rulesFailed,
    onDisk,
    rulesDraft,
    rulesSaving,
    endpointRead,
    endpointList,
    providerModels,
    levels,
    shortcuts,
    choiceBlocked,
    provider,
    modelId,
    level,
    lockBox,
    lockEdited,
    lockField,
    modelWindow,
    share,
    saveRules,
    saveLock,
    saveChoice,
    useTier,
    setRulesDraft,
    setRulesSaving,
    setProvider,
    setModelId,
    setLevel,
    setChoiceDirty,
    setLockBox,
    setLockEdited,
  }: Props = $props();
  const { lang } = ui();
</script>

{#snippet rules()}
  <div class="flex min-w-0 flex-col gap-base">
    {#if rulesFailed !== null}
      <Unanswered
        query={rulesFailed.query}
        reason={rulesFailed.reason}
        asked={{ document: { at: rulesAt(building) } }}
      />
    {:else if onDisk === null}
      <p class="text-text-faint">…</p>
    {:else}
      {#if pane === "models"}
        {#if endpointRead.kind === "unavailable"}
          <Unanswered
            query={endpointRead.query}
            reason={endpointRead.reason}
            asked={QUERIES.endpoints}
          />
        {:else if endpointRead.kind === "asking"}
          <p class="text-text-faint">…</p>
        {:else if endpointList.length === 0}
          <p class="text-note text-text-faint">{say($lang, "subagent_no_endpoint")}</p>
        {:else}
          <div class="flex min-w-0 flex-wrap items-end gap-base">
            <label class="flex min-w-0 flex-col gap-tight text-note">
              <span class="text-text-quiet">{say($lang, "subagent_provider")}</span>
              <select
                class="h-control max-w-full rounded-control border border-edge-input bg-raised px-snug text-note text-text"
                value={provider}
                onchange={(event) => {
                  const name = event.currentTarget.value;
                  setProvider(name);
                  const next = endpointList.find((each) => each.name === name);
                  setModelId(next?.models[0]?.id ?? "");
                  const offered = next?.models[0]?.thinking.levels ?? [];
                  const first = offered[0];
                  setLevel(first ?? "medium");
                  setChoiceDirty(true);
                }}
              >
                <option value=""></option>
                {#each endpointList as endpoint (endpoint.name)}
                  <option value={endpoint.name}>{endpoint.label}</option>
                {/each}
              </select>
            </label>
            <label class="flex min-w-0 flex-col gap-tight text-note">
              <span class="text-text-quiet">{say($lang, "subagent_model")}</span>
              <select
                class="h-control max-w-full rounded-control border border-edge-input bg-raised px-snug text-note text-text"
                value={modelId}
                onchange={(event) => {
                  const id = event.currentTarget.value;
                  setModelId(id);
                  const offered =
                    providerModels.find((each) => each.id === id)?.thinking.levels ?? [];
                  const first = offered[0];
                  if (first === undefined) setLevel("medium");
                  else if (!offered.includes(level)) setLevel(first);
                  setChoiceDirty(true);
                }}
              >
                {#each providerModels as facts (facts.id)}
                  <option value={facts.id}>{facts.id}</option>
                {/each}
              </select>
            </label>
            <label class="flex min-w-0 flex-col gap-tight text-note">
              <span class="text-text-quiet">{say($lang, "subagent_effort")}</span>
              <select
                class="h-control max-w-full rounded-control border border-edge-input bg-raised px-snug text-note text-text"
                value={level}
                onchange={(event) => {
                  const next = levels.find((each) => each === event.currentTarget.value);
                  if (next !== undefined) {
                    setLevel(next);
                    setChoiceDirty(true);
                  }
                }}
              >
                {#each levels as effort (effort)}
                  <option value={effort}>{effort}</option>
                {/each}
              </select>
            </label>
          </div>
          {#if modelId !== "" && levels.length === 0}
            <p class="text-note text-text-faint">{say($lang, "subagent_no_effort")}</p>
          {/if}
          <span class="flex flex-wrap items-center gap-base">
            {#each TIERS as tier (tier)}
              {#if shortcuts[tier] !== undefined}
                <Button
                  label={fill(say($lang, "subagent_shortcut"), { tier })}
                  tone="quiet"
                  onPress={() => {
                    useTier(tier);
                  }}
                />
              {/if}
            {/each}
            <Button
              label={say($lang, "subagent_save")}
              tone="primary"
              loading={rulesSaving.kind === "saving"}
              {...choiceBlocked === undefined ? {} : { why: choiceBlocked }}
              onPress={saveChoice}
            />
            {#if rulesSaving.kind === "saved"}
              <span class="text-note text-text-quiet" role="status"
                >{say($lang, "catalog_saved")}</span
              >
            {:else if rulesSaving.kind === "refused"}
              <span class="text-note text-text-quiet" role="status"
                >{rulesSaving.error.recovery}</span
              >
            {/if}
          </span>
        {/if}
      {/if}
      <TextBox
        size="document"
        wire={textBoxWire(rulesDraft, say($lang, "rules_title"), false, (next) => {
          setRulesDraft(next);
          setRulesSaving(markEdited(next !== onDisk));
        })}
      />
      <span class="flex flex-wrap items-center gap-base">
        <Button
          label={say($lang, "catalog_save_rules")}
          tone="primary"
          {...rulesDraft === onDisk ? { why: say($lang, "catalog_unchanged") } : {}}
          onPress={saveRules}
        />
        {#if rulesSaving.kind === "saved"}
          <span class="text-note text-text-quiet" role="status">{say($lang, "catalog_saved")}</span>
        {:else if rulesSaving.kind === "refused"}
          <span class="text-note text-text-quiet" role="status">{rulesSaving.error.recovery}</span>
        {/if}
      </span>
    {/if}
  </div>
{/snippet}
{#snippet context()}
  <div class="flex min-w-0 flex-col gap-base">
    <Field
      label={say($lang, "catalog_context_lock")}
      kind="number"
      step={1}
      suffix={say($lang, "catalog_context_lock_unit")}
      mono
      placeholder={say($lang, "catalog_context_lock_hint")}
      value={lockBox}
      onInput={(next: string) => {
        setLockBox(next);
        setLockEdited(true);
      }}
    />
    {#if modelWindow === null}
      <p class="text-note text-text-faint">{say($lang, "catalog_context_window_unknown")}</p>
    {:else}
      <p class="text-note text-text-quiet">
        {fill(say($lang, "catalog_context_window"), { window: kilo(modelWindow) })}
        {#if share !== null}
          · {fill(say($lang, "catalog_context_share"), { n: String(share) })}
        {/if}
      </p>
    {/if}
    <p class="text-note text-text-faint">
      {say($lang, "catalog_context_warm")}
      <a
        class="text-accent underline-offset-2 hover:underline"
        href={toFragment({ kind: "setup", group: "accounts" })}
      >
        {say($lang, "nav_settings")}
      </a>
    </p>
    <span class="flex flex-wrap items-center gap-base">
      <Button
        label={say($lang, "catalog_context_save")}
        tone="primary"
        loading={rulesSaving.kind === "saving"}
        {...lockEdited && lockField.kind !== "invalid" && rulesSaving.kind !== "saving"
          ? {}
          : {
              why: say(
                $lang,
                lockField.kind === "invalid"
                  ? "catalog_context_lock_invalid"
                  : rulesSaving.kind === "saving"
                    ? "subagent_saving"
                    : "catalog_unchanged",
              ),
            }}
        onPress={saveLock}
      />
      {#if rulesSaving.kind === "saved"}
        <span class="text-note text-text-quiet" role="status">{say($lang, "catalog_saved")}</span>
      {:else if rulesSaving.kind === "refused"}
        <span class="text-note text-text-quiet" role="status">{rulesSaving.error.recovery}</span>
      {/if}
    </span>
  </div>
{/snippet}
<section class="flex max-w-page min-w-0 flex-col gap-wide" aria-label={say($lang, "bld_catalog")}>
  <h2 class="sr-only">{say($lang, "bld_catalog")}</h2>
  {#if read.kind === "unavailable"}
    <Unanswered query={read.query} asked={question} />
  {:else if look === undefined}
    <p class="text-text-faint">…</p>
  {:else}
    <Look
      {...look}
      rules={pane === "models" || pane === "rules"
        ? rules
        : pane === "context"
          ? context
          : undefined}
    />
    {#if saved}
      <p class="text-note text-text-quiet" role="status">{say($lang, "catalog_saved")}</p>
    {/if}
  {/if}
</section>
