<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts" module>
  export type RowDraws = "everything" | "notice";
</script>

<script lang="ts">
  import { untrack } from "svelte";
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { Address } from "../../wire";
  import type { Choice, Pill } from "./composer";
  import { FILTER_AFTER, splitModel } from "./composer";
  import Listening from "./listening.svelte";
  import { Popover } from "../parts/popover";
  import type { PopoverBinding, PopoverColumn, PopoverRow } from "../parts/popover";
  import PillView, { FACT } from "./pill.svelte";
  import Sandbox from "./sandbox.svelte";
  import { picked, policyColumns, policyFace, POLICY_SWITCHES } from "./policy";

  interface Props {
    readonly specs: readonly [Pill, Pill, Pill];
    readonly room: Address | null;
    readonly draws: RowDraws;
    readonly kept: boolean;
    readonly menu?: "open" | "closed" | "model";
  }

  const PROVIDER_COLUMN = "provider";
  const MODEL_COLUMN = "model";
  const EFFORT_COLUMN = "effort";

  const { specs, room, draws, kept, menu: starts = "closed" }: Props = $props();
  const u = ui();
  const { lang } = u;
  const policy = u.policy;
  let menu = $state<"policy" | "model" | null>(untrack(() => starts) === "open" ? "policy" : untrack(() => starts) === "model" ? "model" : null);
  let provider = $state<string | null>(null);
  let modelTrigger = $state<HTMLButtonElement | null | undefined>(undefined);
  let filter = $state<HTMLInputElement | null | undefined>(undefined);
  let modelQuery = $state("");
  let modelBinding = $state<PopoverBinding | null>(null);
  let modelActive = $state<string | null>(null);
  let modelPreview = untrack(() => starts) === "model";
  let permission = $state<HTMLButtonElement | null | undefined>(undefined);
  let first = $state<HTMLInputElement | null | undefined>(undefined);
  let panel = $state<HTMLDivElement | null | undefined>(undefined);
  let preview = untrack(() => starts) === "open";

  const providers = $derived(specs[0].choices.flatMap((row) => {
    const model = splitModel(row.value);
    return model === null ? [] : [{ id: model.endpoint, label: row.note ?? model.endpoint }];
  }).filter((row, index, all) => all.findIndex((each) => each.id === row.id) === index));
  const parent = $derived(provider ?? splitModel(specs[0].value ?? "")?.endpoint ?? providers.at(0)?.id);
  const children = $derived(modelsFor(parent));
  const needsFilter = $derived(children.length > FILTER_AFTER);
  // The model entry's menu: provider and model are two steps of one
  // table, so its lists stack as rows - provider first, then the models
  // that provider serves, which grow the second row as the first is
  // chosen. Thinking is not a step of that table: it is its own choice,
  // listed apart under a rule (the person's design).
  const columns = $derived<readonly PopoverColumn[]>([
    { id: PROVIDER_COLUMN, label: "talk_column_provider", rows: providers.map((row) => ({ ...row, chosen: row.id === parent })) },
    { id: MODEL_COLUMN, label: "talk_column_model", rows: children.filter((row) => row.label.toLowerCase().includes(modelQuery.trim().toLowerCase())).map((row) => ({ id: row.value, label: row.label, chosen: row.value === specs[0].value })) },
    { id: EFFORT_COLUMN, label: "talk_column_effort", apart: true, rows: specs[2].choices.map((row) => ({ id: row.value, label: row.label, secondary: row.note, chosen: row.value === specs[2].value })) },
  ]);
  const modelFace = $derived(specs[0].choices.find((row) => row.value === specs[0].value)?.label ?? specs[0].placeholder);
  const effortFace = $derived(specs[2].choices.find((row) => row.value === specs[2].value)?.label ?? specs[2].placeholder);

  $effect(() => {
    if (menu !== "policy" || (first === undefined || first === null)) return;
    if (preview) { preview = false; return; }
    first.focus();
  });

  $effect(() => {
    if (menu !== "model" || filter === undefined || filter === null || parent === undefined) return;
    if (modelPreview) { modelPreview = false; return; }
    filter.focus({ preventScroll: true });
  });

  function modelPick(column: PopoverColumn, row: PopoverRow): void {
    modelPreview = false;
    if (column.id === PROVIDER_COLUMN) {
      if (needsFilter && modelsFor(row.id).length <= FILTER_AFTER) modelTrigger?.focus();
      else filter?.focus({ preventScroll: true });
      provider = row.id;
      modelQuery = "";
    }
    if (column.id === MODEL_COLUMN) specs[0].pick(row.id);
    if (column.id === EFFORT_COLUMN) specs[2].pick(row.id);
    if (column.id !== PROVIDER_COLUMN) filter?.focus({ preventScroll: true });
  }

  function modelsFor(endpoint: string | undefined): readonly Choice[] {
    return specs[0].choices.filter((row) => splitModel(row.value)?.endpoint === endpoint);
  }

  function closeModel(): void {
    menu = null;
    modelQuery = "";
    modelBinding = null;
    queueMicrotask(() => { modelTrigger?.focus(); });
  }

  function closePermission(): void {
    menu = null;
    permission?.focus();
  }
</script>

{#snippet modelSearch()}
  <input bind:this={filter} bind:value={modelQuery} type="search"
    aria-label={say($lang, "talk_find_model")} placeholder={say($lang, "talk_find_model")}
    aria-activedescendant={modelActive ?? undefined} aria-controls={modelBinding?.controls.join(" ")}
    class="mb-snug h-control w-full min-w-0 rounded-control bg-page px-snug text-body text-text outline-hidden placeholder:text-text-faint"
    onfocus={() => { modelBinding?.pointColumn(MODEL_COLUMN); }}
    oninput={() => { modelBinding?.pointColumn(MODEL_COLUMN); }}
    onkeydown={(event) => {
      if (event.isComposing || event.key === "Home" || event.key === "End") return;
      if (modelBinding?.keys(event) === true) { event.preventDefault(); event.stopPropagation(); }
    }} />
{/snippet}

{#snippet modelRow(item: PopoverRow)}
  <div class="w-full min-w-0">
    <span class="block wrap-anywhere">{item.label}</span>
    {#if item.secondary !== undefined}
      <span class="mt-tight block wrap-anywhere text-note text-text-faint">{item.secondary}</span>
    {/if}
  </div>
{/snippet}

{#snippet listening()}
  {#if room !== null}<Listening {room} />{/if}
{/snippet}

{#if draws === "everything" || kept}
<div class="relative mt-tight flex items-center justify-between gap-tight">
  <div class="edge-slot -ml-snug flex min-w-0 flex-wrap items-center narrow:ml-0">
    {#if draws === "everything" && specs[1].choices.length > 0}
      <PillView spec={specs[1]} told={room === null ? undefined : listening} />
    {/if}
    {#if draws === "everything"}<Sandbox {room} />{/if}
    {#if kept}<span class="px-snug text-note text-alert">{say($lang, "talk_not_live")}</span>{/if}
  </div>
  {#if draws === "everything"}
    {#if menu === "model" && providers.length > 0}
      <Popover label="talk_column_model" {columns} layout="rows" row={modelRow} onApply={modelPick} onClose={closeModel}
        header={needsFilter ? modelSearch : undefined}
        bind={needsFilter ? (binding: PopoverBinding) => {
          const initial = untrack(() => modelBinding?.controls.at(0) !== binding.controls.at(0));
          modelBinding = binding;
          if (initial) binding.pointColumn(MODEL_COLUMN);
        } : undefined}
        onCursorChange={(id: string | null) => { modelActive = id; }} />
    {/if}
    <div class="-mr-snug ml-auto flex min-w-0 items-center narrow:mr-0">
      {#if providers.length > 0}
        <button bind:this={modelTrigger} type="button" class="{FACT} hover:wash hover:text-text aria-expanded:wash aria-expanded:text-text"
          aria-label={`${specs[0].label}: ${modelFace} | ${effortFace}`} aria-haspopup="dialog" aria-expanded={menu === "model"}
          onclick={() => { modelPreview = false; modelQuery = ""; menu = menu === "model" ? null : "model"; }}>
          <span class="min-w-0 truncate">{modelFace}</span>
          <span aria-hidden="true" class="shrink-0">│</span>
          <span class="shrink-0">{effortFace}</span>
        </button>
      {/if}
      <div bind:this={panel} onfocusout={(event) => {
        if (menu !== "policy") return;
        const next = event.relatedTarget;
        if (next instanceof Node && panel?.contains(next)) return;
        menu = null;
      }}>
        <button bind:this={permission} type="button" class="{FACT} hover:wash hover:text-text aria-expanded:wash aria-expanded:text-text"
          aria-label={`${say($lang, "talk_permissions")}: ${say($lang, `mode_${$policy.mode}`)} · ${policyFace($lang, $policy)}`}
          aria-haspopup="dialog" aria-expanded={menu === "policy"}
          onclick={() => { menu = menu === "policy" ? null : "policy"; }}>
          <span class="truncate">{say($lang, `mode_${$policy.mode}`)} · {policyFace($lang, $policy)}</span>
        </button>
        {#if menu === "policy"}
          <div role="dialog" tabindex="-1" aria-label={say($lang, "talk_permissions")}
            class="rise absolute bottom-full left-0 mb-snug flex w-full flex-col gap-base rounded-panel border border-edge-panel bg-raised p-base shadow-float"
            onkeydown={(event) => {
              if (event.key === "Escape") { event.preventDefault(); event.stopPropagation(); closePermission(); }
            }}>
            <label class="flex items-center justify-between gap-base text-body text-text">
              {say($lang, "talk_work_switch")}
              <input bind:this={first} type="checkbox" role="switch" checked={$policy.mode === POLICY_SWITCHES.mode.on}
                onchange={(event) => { u.choosePolicy({ ...$policy, mode: event.currentTarget.checked ? POLICY_SWITCHES.mode.on : POLICY_SWITCHES.mode.off }); }} />
            </label>
            <label class="flex items-center justify-between gap-base text-body text-text">
              {say($lang, "talk_write_switch")}
              <input type="checkbox" role="switch" checked={$policy.write === POLICY_SWITCHES.write.on}
                onchange={(event) => { u.choosePolicy({ ...$policy, write: event.currentTarget.checked ? POLICY_SWITCHES.write.on : POLICY_SWITCHES.write.off }); }} />
            </label>
            <div class="flex flex-wrap gap-base">
              {#each policyColumns($lang, $policy).filter((column) => column.id !== "write") as column (column.id)}
                <label class="flex min-w-0 flex-1 flex-col gap-tight text-note text-text-quiet">
                  {say($lang, column.label)}
                  <select class="h-control w-full min-w-0 rounded-control bg-page px-snug text-body text-text"
                    value={column.rows.find((row) => row.chosen)?.id}
                    onchange={(event) => { u.choosePolicy(picked($policy, column.id, event.currentTarget.value)); }}>
                    {#each column.rows as row (row.id)}<option value={row.id}>{row.label}</option>{/each}
                  </select>
                </label>
              {/each}
            </div>
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>
{/if}
