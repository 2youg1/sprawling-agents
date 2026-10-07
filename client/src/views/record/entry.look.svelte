<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<!-- How one row of the record's timeline is drawn, and nothing else
(`./entry`, `EntryLook`): time, position, what happened, where, on
column lines every row shares. Under 30rem the line becomes two: the
time and the position, then what happened, whole. A ledger row takes the
wash under the pointer on the arriving curve and gives it back on the
leaving one (docs/frontend-method.md §4-43), and keeps it while it is
open, so the record under it reads as that row's. -->
<script lang="ts">
  import type { EntryLook } from "./entry";

  const look: EntryLook = $props();

  const ROW =
    "grid grid-cols-[14ch_6ch_minmax(0,1fr)] items-baseline gap-x-base py-tight text-note @min-[48rem]:grid-cols-[14ch_6ch_minmax(0,1fr)_minmax(0,24ch)] @max-[30rem]:grid-cols-[14ch_minmax(0,1fr)]";
  const SEQ = "figure text-right text-text-faint @max-[30rem]:text-left";
  const WHAT = "min-w-0 truncate @max-[30rem]:col-span-full @max-[30rem]:whitespace-normal";
  const WHERE = "hidden min-w-0 truncate text-right @min-[48rem]:block";
</script>

{#if look.day !== undefined}
  <li class="flex justify-between border-b border-edge pt-base pb-tight text-note text-text-faint">
    <span class="figure">{look.day.day}</span>
    <span>{look.day.zone}</span>
  </li>
{/if}
{#if look.row.kind === "record"}
  {@const row = look.row}
  <li>
    <button
      {...row.wire}
      class={[
        ROW,
        "w-full rounded-card px-snug text-left transition-colors still:transition-none",
        row.open ? "wash ease-arrive" : "ease-leave hover:wash hover:ease-arrive",
      ]}
    >
      <time class="figure text-text-faint" datetime={row.when.at}>{row.when.text}</time>
      <span class={SEQ}>{row.seq}</span>
      <span class={WHAT}>
        <span class="text-text">{row.what}</span>
        <span class="text-text-quiet">{row.facts}</span>
      </span>
      <span class="{WHERE} font-mono text-text-quiet">{row.where}</span>
    </button>
    {#if row.raw !== undefined}
      <div class="flex flex-col gap-tight pt-tight pb-base pl-[calc(20ch+2*var(--spacing-base))] @max-[30rem]:pl-0">
        <span class="text-note text-text-faint">{row.rawLabel}</span>
        <pre class="max-h-output overflow-auto rounded-card border border-edge p-base text-note text-text-quiet">{row.raw}</pre>
      </div>
    {/if}
  </li>
{:else}
  {@const row = look.row}
  <li class="{ROW} px-snug font-mono">
    {#if row.when === undefined}
      <span class="text-text-faint"></span>
    {:else}
      <time class="figure text-text-faint" datetime={row.when.at}>{row.when.text}</time>
    {/if}
    <span class={SEQ}>{row.seq}</span>
    <span class={WHAT}>
      <span class="text-text-faint">{row.level}</span>
      <span class="text-text-quiet">{row.line}</span>
    </span>
    <span class="{WHERE} text-text-faint">{row.module}</span>
  </li>
{/if}
