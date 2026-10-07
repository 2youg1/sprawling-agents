<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // How the uses are drawn (`./usage_uses`, `UsesLook`): the count, the
  // days, and every use folded under a disclosure.
  import type { UsesLook } from "./usage_uses";

  const look: UsesLook = $props();
</script>

{#if look.kind === "never"}
  <p class="text-note text-text-faint">{look.said}</p>
{:else}
  <p class="text-note text-text-quiet">{look.count}</p>
  <p class="font-mono text-note text-text-faint">{look.days}</p>
  <details class="text-note">
    <summary class="cursor-pointer text-text-faint hover:text-text">{look.fold}</summary>
    <ul class="mt-tight flex flex-col gap-tight">
      {#each look.uses as used, index (index)}
        <li class="flex min-w-0 items-baseline gap-base text-text-faint">
          <span class="shrink-0">{used.at}</span>
          <span class="min-w-0 truncate font-mono">{used.who}</span>
          {#if used.part !== undefined}
            <span class="min-w-0 truncate font-mono">{used.part}</span>
          {/if}
          <span class="shrink-0">{used.outcome}</span>
        </li>
      {/each}
    </ul>
  </details>
{/if}
