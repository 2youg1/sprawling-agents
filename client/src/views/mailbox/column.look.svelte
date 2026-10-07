<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // How the mailbox column is drawn (client/Spec.lean §4-49): a head
  // one bar tall with the name, the link's word and its recovery, and
  // under it one scroller holding every section. Beside each of the
  // first nine entries stands the digit that reaches it.
  import Button from "../parts/button.svelte";
  import type { ColumnLook } from "./column";

  const look: ColumnLook = $props();
</script>

<div class="flex h-full min-h-0 flex-col">
  <header class="flex h-bar shrink-0 items-center gap-snug border-b border-edge px-base">
    <span class="hidden narrow:inline-flex">
      <Button tone="quiet" label={look.back.label} onPress={look.back.onPress} />
    </span>
    <h2 class="min-w-0 flex-1 truncate text-label font-label">{look.title}</h2>
    {#if look.link !== undefined}
      <span class="shrink-0 text-note text-text-faint">{look.link.word}</span>
      {#if look.link.recovery !== undefined}
        <Button tone="quiet" label={look.link.recovery.label} onPress={look.link.recovery.onPress} />
      {/if}
    {/if}
  </header>
  <div class="entries min-h-0 flex-1 overflow-y-auto px-base pb-wide" {...look.scroller}>
    {@render look.body()}
  </div>
</div>

<style>
  /* The digit beside each of the first nine entries (client/Spec.lean
   * §7-11): the column counts every `data-entry` mark in document order,
   * which is the order `./entries.ts` walks, so the number drawn is the
   * number a digit reaches. Past nine the slot is empty; a slot outside
   * the column is never counted and draws nothing. */
  @counter-style mailbox-blank {
    system: cyclic;
    symbols: "";
  }
  @counter-style mailbox-digit {
    system: extends decimal;
    range: 1 9;
    fallback: mailbox-blank;
  }
  .entries {
    counter-reset: entry;
  }
  .entries :global([data-entry]) {
    counter-increment: entry;
  }
  .entries :global(.entry-n) {
    flex-shrink: 0;
    min-width: 1ch;
    margin-inline-start: var(--spacing-snug);
    font-family: var(--font-mono);
    font-size: var(--text-note);
    line-height: 1;
    color: var(--color-text-faint);
  }
  .entries :global(.entry-n)::before {
    content: counter(entry, mailbox-digit);
  }
</style>
