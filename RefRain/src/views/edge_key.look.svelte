<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  import type { Snippet } from "svelte";

  import type { EdgeKeyLook } from "./edge";
  import Glyph from "./parts/glyph.svelte";
  import Tip from "./parts/tip.svelte";

  const look: EdgeKeyLook = $props();

  const key: Snippet<[string]> = drawKey;
  const inside: Snippet = drawInside;
</script>

{#snippet drawInside()}
  <Glyph name={look.glyph} size="key" />
  {#if look.foot.kind === "tiers"}
    <span class="foot" aria-hidden="true">
      {#each look.foot.ticks as tick (tick.key)}
        <i class="tick" data-held={tick.held}></i>
      {/each}
    </span>
  {:else if look.foot.kind === "link"}
    <span class="foot" aria-hidden="true">
      <i class={["bar", look.foot.link === "connecting" && "pulse"]} data-link={look.foot.link}></i>
    </span>
  {/if}
  {#if look.corner.kind === "count"}
    <span class="count" aria-hidden="true">{look.corner.text}</span>
  {:else if look.corner.kind === "fresh"}
    <span class="fresh" aria-hidden="true"></span>
  {/if}
{/snippet}

{#snippet drawKey(hint: string)}
  {#if look.key.as === "button"}
    <button class="key glass" {...look.key.wire} aria-describedby={hint}>{@render inside()}</button>
  {:else}
    <a class="key glass" {...look.key.wire} aria-describedby={hint}>{@render inside()}</a>
  {/if}
{/snippet}

<span class="edge">
  <Tip text={look.hint} side="right" exposable>
    {#snippet children(hint: string)}{@render key(hint)}{/snippet}
  </Tip>
</span>

<style>
  .edge {
    display: contents;
  }
  :global(:root[data-expose]) .edge :global([data-exposable]) {
    display: block;
    opacity: 1;
    transition-delay: 0ms;
  }
  .key {
    position: relative;
    display: grid;
    place-items: center;
    width: var(--spacing-key);
    height: var(--spacing-key);
    border-radius: var(--radius-key);
    color: var(--color-text);
    transition: translate var(--transition-duration-short) var(--ease-leave);
  }
  .key:hover {
    translate: 0 -1px;
    transition-timing-function: var(--ease-arrive);
  }

  .foot {
    position: absolute;
    bottom: calc((var(--spacing-key) - var(--spacing-glyph-key)) / 4);
    left: 50%;
    display: flex;
    gap: var(--spacing-hair);
    translate: -50% 0;
  }
  .tick,
  .bar {
    display: block;
    height: var(--spacing-hair);
    border-radius: var(--radius-pill);
  }
  .tick {
    width: var(--spacing-tick);
    background-color: var(--color-mark);
  }
  .tick[data-held="true"] {
    background-color: var(--color-accent);
  }

  .bar {
    width: calc(var(--spacing-tick) * 3 + var(--spacing-hair) * 2);
    background-color: var(--color-text-quiet);
  }
  .bar[data-link="refused"] {
    background-color: var(--color-alert);
  }

  .count {
    position: absolute;
    top: calc(var(--spacing-tight) * -1);
    right: calc(var(--spacing-tight) * -1);
    display: grid;
    place-items: center;
    min-width: var(--spacing-glyph-key);
    height: var(--spacing-glyph-key);
    padding-inline: var(--spacing-tight);
    border-radius: var(--radius-pill);
    background-color: var(--color-alert);
    color: var(--color-on-accent);
    font-size: var(--text-tally);
    font-weight: var(--font-weight-label);
    line-height: 1;
  }
  .fresh {
    position: absolute;
    top: calc(var(--spacing-tight) * -1 + (var(--spacing-glyph-key) - var(--spacing-dot)) / 2);
    right: calc(var(--spacing-tight) * -1 + (var(--spacing-glyph-key) - var(--spacing-dot)) / 2);
    width: var(--spacing-dot);
    height: var(--spacing-dot);
    border-radius: var(--radius-pill);
    background-color: var(--color-text-quiet);
  }
</style>
