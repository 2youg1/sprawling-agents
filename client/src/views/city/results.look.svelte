<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // How the results city's runs are drawn, and nothing else
  // (`./results.ts`, `ResultsLook`): a section per recency band under a
  // ruled heading with its count, and in it one row per run - when it
  // began, how it ended, its room, its task, and the end the judge
  // chose. A run waiting for the person carries the alert edge on its
  // left, so the rows that need an answer are found by their edge.
  //
  // A row is a row of the page, so the pointer lays the page's wash over
  // it rather than raising a surface (docs/frontend-method.md §7A-4);
  // the wash arrives on the theme's arrive curve and leaves on leave
  // (§4-43).
  import Glyph from "../parts/glyph.svelte";
  import { OUTCOME_GLYPH, OUTCOME_INK } from "../shared/outcome";
  import Produced from "./produced.svelte";
  import type { ResultsLook } from "./results";

  const look: ResultsLook = $props();
</script>

<div class="flex flex-col gap-wide">
{#if look.none !== null}
  <p class="text-note text-text-faint">{look.none}</p>
{/if}
{#each look.bands as band (band.key)}
  <section aria-label={band.heading}>
    <h2 class="mb-tight border-b border-edge pb-tight text-note text-text-quiet">
      {band.heading}
      <span class="text-text-faint">{band.count}</span>
    </h2>
    <ul>
      {#each band.rows as row (row.key)}
        <li class="border-b border-l-2 border-b-edge {row.outcome === 'waiting' ? 'border-l-alert' : 'border-l-transparent'}">
          <a
            {...row.wire}
            class="flex items-baseline gap-base px-tight py-snug text-note transition-colors ease-leave hover:wash hover:ease-arrive"
          >
            <span class="w-figure shrink-0 font-mono text-text-faint">{row.at}</span>
            {#if row.outcome !== null}
              <span class="shrink-0 self-center {OUTCOME_INK[row.outcome]}"><Glyph name={OUTCOME_GLYPH[row.outcome]} size="sm" /></span>
            {/if}
            <span class="w-output shrink-0 truncate font-mono text-text-quiet">{row.addr}</span>
            <span class="min-w-0 flex-1 truncate text-body text-text">{row.title}</span>
            {#if row.tail.kind === "produced"}
              <Produced run={row.tail.run} />
              {#if row.tail.pr !== null}
                <span class="shrink-0 text-text-quiet">{row.tail.pr}</span>
              {/if}
            {:else if row.tail.kind === "asks"}
              <span class="min-w-0 shrink truncate text-text">{row.tail.text}</span>
            {:else if row.tail.kind === "stopped"}
              <span class="shrink-0 text-text-quiet">{row.tail.text}</span>
            {/if}
          </a>
        </li>
      {/each}
    </ul>
  </section>
{/each}
</div>
