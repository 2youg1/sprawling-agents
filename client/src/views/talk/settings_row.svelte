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
  import type { Pill } from "./composer";
  import { splitModel } from "./composer";
  import Listening from "./listening.svelte";
  import { Popover } from "../parts/popover";
  import type { PopoverColumn, PopoverRow } from "../parts/popover";
  import PillView, { FACT } from "./pill.svelte";
  import { picked, policyColumns, policyFace } from "./policy";

  interface Props {
    readonly specs: readonly [Pill, Pill, Pill];
    readonly room: Address | null;
    readonly draws: RowDraws;
    readonly kept: boolean;
    readonly menu?: "open" | "closed" | "model";
  }

  const { specs, room, draws, kept, menu: starts = "closed" }: Props = $props();
  const u = ui();
  const { lang } = u;
  const policy = u.policy;
  let menu = $state<"policy" | "model" | null>(untrack(() => starts) === "open" ? "policy" : untrack(() => starts) === "model" ? "model" : null);
  let provider = $state<string | null>(null);
  let permission = $state<HTMLButtonElement | undefined>(undefined);
  let first = $state<HTMLInputElement | undefined>(undefined);
  let panel = $state<HTMLDivElement | undefined>(undefined);
  let preview = untrack(() => starts) === "open";

  const providers = $derived(specs[0].choices.flatMap((row) => {
    const model = splitModel(row.value);
    return model === null ? [] : [{ id: model.endpoint, label: row.note ?? model.endpoint }];
  }).filter((row, index, all) => all.findIndex((each) => each.id === row.id) === index));
  const parent = $derived(provider ?? splitModel(specs[0].value ?? "")?.endpoint ?? providers.at(0)?.id);
  const columns = $derived<readonly PopoverColumn[]>([
    { id: "provider", label: "talk_column_provider", rows: providers.map((row) => ({ ...row, chosen: row.id === parent })) },
    { id: "model", label: "talk_column_model", rows: specs[0].choices.filter((row) => splitModel(row.value)?.endpoint === parent).map((row) => ({ id: row.value, label: row.label, chosen: row.value === specs[0].value })) },
    { id: "effort", label: "talk_column_effort", rows: specs[2].choices.map((row) => ({ id: row.value, label: row.label, secondary: row.note, chosen: row.value === specs[2].value })) },
  ]);
  const modelFace = $derived(specs[0].choices.find((row) => row.value === specs[0].value)?.label ?? specs[0].placeholder);
  const effortFace = $derived(specs[2].choices.find((row) => row.value === specs[2].value)?.label ?? specs[2].placeholder);

  $effect(() => {
    if (menu !== "policy" || first === undefined) return;
    if (preview) { preview = false; return; }
    first.focus();
  });

  function modelPick(column: PopoverColumn, row: PopoverRow): void {
    if (column.id === "provider") provider = row.id;
    if (column.id === "model") specs[0].pick(row.id);
    if (column.id === "effort") specs[2].pick(row.id);
  }

  function closePermission(): void {
    menu = null;
    permission?.focus();
  }
</script>

{#snippet listening()}
  {#if room !== null}<Listening {room} />{/if}
{/snippet}

{#if draws === "everything" || kept}
<div class="relative mt-tight flex items-center justify-between gap-tight">
  <div class="edge-slot -ml-snug flex min-w-0 flex-wrap items-center narrow:ml-0">
    {#if draws === "everything" && specs[1].choices.length > 0}
      <PillView spec={specs[1]} told={room === null ? undefined : listening} />
    {/if}
    {#if kept}<span class="px-snug text-note text-alert">{say($lang, "talk_not_live")}</span>{/if}
  </div>
  {#if draws === "everything"}
    {#if menu === "model" && providers.length > 0}
      <Popover label="talk_column_model" {columns} onApply={modelPick} onClose={() => { menu = null; }} />
    {/if}
    <div class="-mr-snug ml-auto flex min-w-0 items-center narrow:mr-0">
      {#if providers.length > 0}
        <button type="button" class="{FACT} hover:wash hover:text-text aria-expanded:wash aria-expanded:text-text"
          aria-label={`${specs[0].label}: ${modelFace} | ${effortFace}`} aria-haspopup="dialog" aria-expanded={menu === "model"}
          onclick={() => { menu = menu === "model" ? null : "model"; }}>
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
              <input bind:this={first} type="checkbox" role="switch" checked={$policy.mode === "work"}
                onchange={(event) => { u.choosePolicy({ ...$policy, mode: event.currentTarget.checked ? "work" : "chat" }); }} />
            </label>
            <label class="flex items-center justify-between gap-base text-body text-text">
              {say($lang, "talk_write_switch")}
              <input type="checkbox" role="switch" checked={$policy.write === "full"}
                onchange={(event) => { u.choosePolicy({ ...$policy, write: event.currentTarget.checked ? "full" : "create" }); }} />
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
