<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- How the settings row is drawn, and nothing else (client D95): the
facts on the left, the model picker's token and the permissions entry
on the right, and the panel each opens above the row. The facts' text
hangs one padding outside the row's edges, so the words line up with
the composer's line above them on a wide column; on one column the row
keeps its padding. The token is one control in three segments - model,
provider, thinking level - each marked with the section a press on it
opens; the open picker is its own seat, `picker.svelte`. Every word arrives translated and every role, key
handler and `aria-*` value arrives in a wire bag spread on the element
it is for. -->
<script lang="ts">
  import Fact from "./fact.look.svelte";
  import Picker from "./picker.svelte";
  import type { SettingsRowLook } from "./settings_row";

  const look: SettingsRowLook = $props();
</script>

<div class="relative flex items-center justify-between gap-tight pt-tight">
  <div class="edge-slot -ml-snug flex min-w-0 flex-wrap items-center narrow:ml-0">
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
    {#if look.facts !== undefined}{@render look.facts()}{/if}
    {#if look.notLive !== undefined}<span class="px-snug text-note text-alert">{look.notLive}</span>{/if}
  </div>
  {#if look.model !== undefined || look.permissions !== undefined}
    <div class="-mr-snug ml-auto flex min-w-0 items-center narrow:mr-0">
      {#if look.model !== undefined}
        {@const picker = look.model}
        <div class="contents" {...picker.frame}>
          {#if picker.menu !== undefined}<Picker menu={picker.menu} />{/if}
          <button type="button"
            class="token inline-flex h-touch min-w-0 max-w-full items-center rounded-control text-note text-text-quiet hover:text-text aria-expanded:text-text"
            {...picker.token}>
            {#each picker.segments as part, at (part.segment)}
              {#if at > 0}<span aria-hidden="true" class="shrink-0 text-text-faint">·</span>{/if}
              <span data-segment={part.segment}
                class={["inline-flex h-touch min-w-0 items-center rounded-control px-snug hover:wash", part.open && "wash", part.segment === "model" ? "truncate" : "shrink-0"]}
                >{part.label}</span>
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
