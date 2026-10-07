<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- How the settings row is drawn, and nothing else (client D95): the
facts on the left, the two entries on the right, each a fact of the row
in its one shape (`fact.look.svelte`), and the panel each opens above
the row. The facts' text hangs one padding outside the row's edges, so
the words line up with the composer's line above them on a wide column;
on one column the row keeps its padding. Every word arrives translated
and every role, key handler and `aria-*` value arrives in a wire bag
spread on the element it is for; the model panel is the shared popover
part, given the search box and the row drawing from here. -->
<script lang="ts">
  import { Popover } from "../parts/popover";
  import type { PopoverRow } from "../parts/popover";
  import Fact from "./fact.look.svelte";
  import type { SettingsRowLook } from "./settings_row";

  const look: SettingsRowLook = $props();
</script>

{#snippet search()}
  {#if look.model?.search !== undefined}
    <input class="mb-snug h-control w-full min-w-0 rounded-control bg-page px-snug text-body text-text outline-hidden placeholder:text-text-faint"
      {...look.model.search} />
  {/if}
{/snippet}

{#snippet modelRow(item: PopoverRow)}
  <div class="w-full min-w-0">
    <span class="block wrap-anywhere">{item.label}</span>
    {#if item.secondary !== undefined}
      <span class="mt-tight block wrap-anywhere text-note text-text-faint">{item.secondary}</span>
    {/if}
  </div>
{/snippet}

<div class="relative flex items-center justify-between gap-tight pt-tight">
  <div class="edge-slot -ml-snug flex min-w-0 flex-wrap items-center narrow:ml-0">
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
    {#if look.facts !== undefined}{@render look.facts()}{/if}
    {#if look.notLive !== undefined}<span class="px-snug text-note text-alert">{look.notLive}</span>{/if}
  </div>
  {#if look.model !== undefined || look.permissions !== undefined}
    {#if look.model?.menu !== undefined}
      <Popover {...look.model.menu} header={look.model.search === undefined ? undefined : search} row={modelRow} />
    {/if}
    <div class="-mr-snug ml-auto flex min-w-0 items-center narrow:mr-0">
      {#if look.model !== undefined}
        <Fact wire={look.model.trigger}>
          <span class="min-w-0 truncate">{look.model.model}</span>
          <span aria-hidden="true" class="shrink-0">│</span>
          <span class="shrink-0">{look.model.effort}</span>
        </Fact>
      {/if}
      {#if look.permissions !== undefined}
        {@const permissions = look.permissions}
        <div class="flex min-w-0" {...permissions.frame}>
          <Fact wire={permissions.trigger}>
            <span class="truncate">{permissions.face}</span>
          </Fact>
          {#if permissions.panel !== undefined}
            <div class="rise absolute bottom-full left-0 mb-snug flex w-full flex-col gap-base rounded-panel border border-edge-panel bg-raised p-base shadow-float"
              {...permissions.panel.wire}>
              {#each permissions.panel.switches as each (each.key)}
                <label class="flex items-center justify-between gap-base text-body text-text">
                  {each.label}
                  <input {...each.wire} />
                </label>
              {/each}
              <div class="flex flex-wrap gap-base">
                {#each permissions.panel.selects as each (each.key)}
                  <label class="flex min-w-0 flex-1 flex-col gap-tight text-note text-text-quiet">
                    {each.label}
                    <select class="h-control w-full min-w-0 rounded-control bg-page px-snug text-body text-text" {...each.wire}>
                      {#each each.options as option (option.key)}<option value={option.key}>{option.label}</option>{/each}
                    </select>
                  </label>
                {/each}
              </div>
            </div>
          {/if}
        </div>
      {/if}
    </div>
  {/if}
</div>
