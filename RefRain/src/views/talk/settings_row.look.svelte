<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  import Fact from "./fact.look.svelte";
  import Picker from "./picker.svelte";
  import type { SettingsRowLook } from "./settings_row";
  const look: SettingsRowLook = $props();
</script>

<div class="relative flex flex-wrap items-center justify-between gap-tight pt-tight">
  <div class="edge-slot -ml-snug flex min-w-0 flex-wrap items-center narrow:ml-0">
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
    {#if look.catalog !== undefined}{@render look.catalog()}{/if}
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
    {#if look.facts !== undefined}{@render look.facts()}{/if}
    {#if look.notLive !== undefined}<span class="px-snug text-note text-alert">{look.notLive}</span
      >{/if}
  </div>
  {#if look.model !== undefined || look.permissions !== undefined}
    <div class="-mr-snug ml-auto flex min-w-0 items-center narrow:mr-0">
      {#if look.model !== undefined}
        {@const picker = look.model}
        <div class="contents" {...picker.frame}>
          {#if picker.menu !== undefined}<Picker menu={picker.menu} />{/if}
          <button
            type="button"
            class="token inline-flex h-touch min-w-0 max-w-full items-center rounded-control text-note text-text-quiet transition-[background-color,color,scale] ease-leave hover:bg-raised-hover hover:text-text hover:ease-arrive active:scale-(--scale-press) active:ease-arrive still:transition-none still:active:scale-100 aria-expanded:text-text"
            {...picker.token}
          >
            {#each picker.segments as part, at (part.segment)}
              {#if at > 0}<span aria-hidden="true" class="shrink-0 text-text-faint">·</span>{/if}
              <span
                data-segment={part.segment}
                class={[
                  "inline-flex h-touch min-w-0 items-center rounded-control px-snug hover:wash",
                  part.open && "wash",
                  part.segment === "model" ? "" : "shrink-0",
                ]}><span class="truncate">{part.label}</span></span
              >
            {/each}
          </button>
        </div>
      {/if}
      {#if look.permissions !== undefined}
        {@const permissions = look.permissions}
        <div class="flex min-w-0" {...permissions.frame}>
          <Fact wire={permissions.trigger}>
            <span class="truncate">{permissions.face}</span>
          </Fact>
          {#if permissions.panel !== undefined}
            <div
              class="rise absolute bottom-full left-0 z-(--layer-popover) mb-snug flex w-full flex-col gap-base rounded-panel border border-edge-panel bg-raised p-base shadow-float"
              {...permissions.panel.wire}
            >
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
                    <select
                      class="h-control w-full min-w-0 rounded-control bg-page px-snug text-body text-text"
                      {...each.wire}
                    >
                      {#each each.options as option (option.key)}<option value={option.key}
                          >{option.label}</option
                        >{/each}
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
