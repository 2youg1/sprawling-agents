<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- How a session row's menu is drawn: a small square key with the
"more" mark, and under its right edge a raised box holding either the
items or the one field the menu turned into. While the key cannot be
pressed, a hint says why on hover. Every role, key and `aria-*` value
comes in the bags. -->
<script lang="ts">
  import Glyph from "../parts/glyph.svelte";
  import Tip from "../parts/tip.svelte";
  import type { SessionMenuLook } from "./session_menu";

  const look: SessionMenuLook = $props();

  // Trigger: wash on hover, yield on press, return on release.
  const KEY =
    "grid size-control-sm place-items-center rounded-control text-text-quiet " +
    "transition-[background-color,color,scale] ease-leave " +
    "hover:wash hover:text-text hover:ease-arrive " +
    "active:scale-(--scale-press) active:ease-arrive " +
    "aria-expanded:wash aria-expanded:text-text " +
    "disabled:text-text-disabled still:transition-none still:active:scale-100";
  // Long policy lines wrap instead of crushing into one control height;
  // the list scrolls under a thin scrollbar once it outgrows the palette.
  const ITEM =
    "flex min-h-control-sm w-full items-center rounded-control px-snug py-tight text-left text-note text-pretty text-text " +
    "transition-[background-color,color] ease-leave " +
    "hover:wash hover:ease-arrive focus-visible:wash active:wash-strong active:ease-arrive";
</script>

<div class="relative">
  {#if look.why === undefined}
    <button {...look.trigger} class={KEY}><Glyph name="more" size="sm" /></button>
  {:else}
    <Tip text={look.why}>
      <button {...look.trigger} class={KEY}><Glyph name="more" size="sm" /></button>
    </Tip>
  {/if}
  {#if look.open}
    <div
      {...look.popup}
      class="absolute top-full right-0 z-(--layer-popover) flex w-max min-w-[24ch] max-w-[36ch] flex-col rounded-card bg-raised p-tight shadow-float"
    >
      {#if look.naming !== undefined}
        <form {...look.naming.form} class="flex flex-col gap-tight p-tight">
          <input
            {...look.naming.field}
            class="h-control-sm rounded-control bg-page px-snug text-note text-text"
          />
          <p
            id={look.naming.hint.id}
            class={["text-note", look.naming.hint.wrong ? "text-alert" : "text-text-faint"]}
          >
            {look.naming.hint.text}
          </p>
        </form>
      {:else}
        <ul {...look.list} class="max-h-palette overflow-y-auto">
          {#each look.items as item (item.key)}
            <li {...item.holder}>
              <button {...item.wire} class={ITEM}>
                {item.word}
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  {/if}
</div>
