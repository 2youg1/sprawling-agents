<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- How what an approval card asks about is drawn (`AskedLook`,
`./asked`): an open fold on the card, the text bounded like a tool's
output in the mono face on the page's own surface, and a line saying
what the cut left out. -->
<script lang="ts">
  import type { AskedLook } from "./asked";

  const { summary, shown }: AskedLook = $props();
</script>

<details class="text-note" open>
  <summary class="fold cursor-pointer text-text-faint">{summary}</summary>
  {#if shown.kind === "asking"}
    <p class="text-text-faint">…</p>
  {:else if shown.kind === "binary"}
    <p class="text-text-faint">{shown.said}</p>
  {:else}
    <pre
      class="mt-tight max-h-output overflow-auto rounded-card border border-edge bg-page p-snug font-mono text-note whitespace-pre-wrap text-text-quiet">{shown.text}</pre>
    {#if shown.cut !== undefined}
      <p class="text-text-faint">{shown.cut}</p>
    {/if}
  {/if}
</details>

<style>
  .fold {
    transition: color var(--transition-duration-short) var(--ease-leave);
  }
  .fold:hover {
    color: var(--color-text-quiet);
    transition-timing-function: var(--ease-arrive);
  }
</style>
