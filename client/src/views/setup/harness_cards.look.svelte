<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // How the harness cards are drawn, and nothing else: every word
  // arrives translated (`./harnesses`, `HarnessCardLook`), so another
  // look draws the same cards by taking the same value.
  import Badge from "../parts/badge.svelte";
  import type { HarnessCardLook } from "./harnesses";

  const { cards }: { readonly cards: readonly HarnessCardLook[] } = $props();
</script>

<ul class="flex flex-col gap-base">
  {#each cards as card (card.key)}
    <li class="flex flex-col gap-tight rounded-card bg-raised px-base py-snug">
      <div class="flex items-center gap-snug">
        <span class="text-label font-label text-text">{card.name}</span>
        <Badge text={card.state} status={card.status} />
      </div>
      {#if card.said !== null}
        <span class="min-w-0 break-all font-mono text-note text-text-faint">{card.said}</span>
      {/if}
      {#if card.lookedIn !== undefined}
        <div class="flex min-w-0 flex-col gap-tight text-note">
          <span class="text-text-quiet">{card.lookedIn}</span>
          <ul class="flex min-w-0 flex-col">
            {#each card.looked as path (path)}
              <li class="min-w-0 break-all font-mono text-text-faint">{path}</li>
            {/each}
          </ul>
        </div>
      {/if}
      <span class="font-mono text-note text-text-quiet">{card.launch}</span>
      <a href={card.docs.href} target="_blank" rel="noopener noreferrer" class="sign-in w-fit text-note">
        {card.docs.label}
      </a>
    </li>
  {/each}
</ul>

<style>
  /* The sign-in link reads as a link at rest and answers the pointer:
   * the underline thickens arriving on the decelerating curve and thins
   * leaving on the accelerating one (docs/frontend-method.md §4-43). */
  .sign-in {
    color: var(--color-accent);
    text-decoration-line: underline;
    text-decoration-thickness: 1px;
    text-underline-offset: 0.2em;
    transition: text-decoration-thickness var(--transition-duration-short) var(--ease-leave);
  }
  .sign-in:hover {
    text-decoration-thickness: var(--spacing-hair);
    transition-timing-function: var(--ease-arrive);
  }
</style>
