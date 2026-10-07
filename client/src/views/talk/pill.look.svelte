<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- How a pill under the box is drawn, and nothing else (client D95):
the trigger is a fact of the settings row showing the value alone, and
the menu opens above it, because the box sits at the foot of the window.
The menu is as wide as its longest row rather than as wide as the
trigger: every row is a short name over one line saying what it changes,
and a row cut off at the trigger's width is a row nobody can choose by
reading it. Every word arrives translated and every role, key handler
and `aria-*` value arrives in a wire bag spread on the element it is
for. -->
<script lang="ts">
  import Glyph from "../parts/glyph.svelte";
  import Fact from "./fact.look.svelte";
  import type { PillLook } from "./pill";

  const look: PillLook = $props();
</script>

<div class="relative" {...look.frame}>
  <Fact wire={look.trigger}>
    <span class="truncate">{look.face}</span>
  </Fact>
  {#if look.menu !== undefined}
    {@const menu = look.menu}
    <div class={["menu rise absolute bottom-full z-20 mb-tight flex w-max flex-col rounded-panel border border-edge-panel bg-raised p-tight shadow-float", menu.edge === "left" ? "left-0" : "right-0"]}
      {...menu.box}>
      <p class="px-base pt-tight pb-snug text-note text-text-faint">{menu.about}</p>
      {#if menu.told !== undefined}
        <div id={menu.told.id} class="mb-tight border-b border-edge">{@render menu.told.says()}</div>
      {/if}
      {#if menu.filter !== undefined}
        <input class="mb-tight h-control w-full rounded-control bg-page px-base text-body text-text placeholder:text-text-faint"
          {...menu.filter} />
      {/if}
      <ul class="list overflow-y-auto" {...menu.list}>
        {#each menu.options as option (option.key)}
          <!-- The row is picked by pointer; the key table belongs to the
              list, which holds the focus (the wire bag carries both). -->
          <li class={["row flex cursor-pointer items-start gap-snug rounded-control px-base py-snug", option.cursor ? "bg-raised-hover text-text" : "text-text-quiet"]}
            {...option.wire}>
            <span class="flex min-w-0 flex-1 flex-col">
              <span class="text-body break-words">{option.label}</span>
              {#if option.note !== undefined}
                <span class="text-note break-words text-text-faint">{option.note}</span>
              {/if}
            </span>
            <span class="mt-tight size-glyph-sm shrink-0">
              {#if option.chosen}<Glyph name="check" size="sm" class="text-text-quiet" />{/if}
            </span>
          </li>
        {:else}
          <li role="presentation" class="px-base py-snug text-note text-text-faint">{menu.empty}</li>
        {/each}
      </ul>
    </div>
  {/if}
</div>

<style>
  /* The menu takes its longest row's width, between one fact's width and
   * the window less a margin on each side. */
  .menu {
    min-width: 16rem;
    max-width: min(28rem, calc(100vw - 2rem));
  }

  /* Most of the window's height, and never so tall that a long list
   * hides the box it opened from. */
  .list {
    max-height: min(70vh, 36rem);
  }

  /* The cursor's lift fades out as it leaves a row and arrives on the
   * next (docs/frontend-method.md §4-43). */
  .row {
    transition-property: background-color, color;
    transition-duration: var(--transition-duration-short);
    transition-timing-function: var(--ease-leave);
  }
  .row.bg-raised-hover {
    transition-timing-function: var(--ease-arrive);
  }
</style>
