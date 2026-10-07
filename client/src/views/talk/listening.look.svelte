<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- How who is listening is drawn (client D95): a heading, then one
term per listener with who it is, the size of its segment, what the
budget cut off it and the files it was read from. Nothing here is a
control; the menu's keys belong to its list. -->
<script lang="ts">
  import type { ListeningLook } from "./listening";

  const look: ListeningLook = $props();
</script>

<div class="px-base pb-snug text-note">
  <p class="pb-tight text-text-faint">{look.heading}</p>
  <dl class="listeners grid gap-x-base gap-y-tight">
    {#each look.listeners as listener (listener.key)}
      <dt class="text-text-faint">{listener.word}</dt>
      <dd class="flex min-w-0 flex-col">
        <span class="flex min-w-0 items-baseline gap-base">
          {#if listener.who !== undefined}<span class="truncate text-text">{listener.who}</span>{/if}
          {#if listener.size !== undefined}<span class="figure shrink-0 text-text-faint">{listener.size}</span>{/if}
          {#if listener.cut !== undefined}<span class="shrink-0 text-alert">{listener.cut}</span>{/if}
        </span>
        {#if listener.sources !== undefined}<span class="truncate text-text-faint">{listener.sources}</span>{/if}
      </dd>
    {/each}
  </dl>
  {#if look.note !== undefined}<p class="pt-tight text-text-faint">{look.note}</p>{/if}
</div>

<style>
  /* The words that name the listeners take their own width and the rest
   * of the row goes to what each was told. */
  .listeners {
    grid-template-columns: max-content minmax(0, 1fr);
  }
</style>
