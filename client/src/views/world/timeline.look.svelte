<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- How the chosen session's timeline is drawn inside the section its
seat lays out: a quiet head with the
day, then one row per turn, call and checkpoint, the list fading out at
its foot. A row is one line where the pane is wide enough for its four
cells - instant, kind, subject, measure - and two where it is not: the
instant and the measure above, the kind and the subject under them. A
cell never wraps inside itself. A call and a checkpoint are buttons that
wash on hover; the picked checkpoint is washed and carries the chosen
bar. Every role and `aria-*` value comes in the bags. -->
<script lang="ts">
  import Produced from "../talk/produced.svelte";
  import type { TimelineLook } from "./timeline";

  const look: TimelineLook = $props();

  const ROW =
    "grid grid-cols-[8ch_minmax(0,1fr)_auto] items-center gap-x-base rounded-card px-snug -mx-snug py-tight text-note whitespace-nowrap @min-cells:h-control @min-cells:grid-cols-[14ch_8ch_minmax(0,1fr)_auto] @min-cells:py-0";
  const AT = "figure col-span-2 text-text-faint @min-cells:col-span-1";
  const KIND = "row-start-2 col-start-1 truncate @min-cells:row-start-1 @min-cells:col-start-2";
  const SUBJECT = "row-start-2 col-start-2 col-span-2 truncate @min-cells:row-start-1 @min-cells:col-start-3 @min-cells:col-span-1";
  const MEASURE = "figure row-start-1 col-start-3 justify-self-end text-text-faint @min-cells:col-start-4";
</script>

<h3 class="flex shrink-0 justify-between py-snug text-note text-text-faint">
  <span>{look.title}</span>
  {#if look.day !== ""}<span class="figure">{look.day}</span>{/if}
</h3>
<ol {...look.list} class="fade min-h-0 flex-1 overflow-y-auto">
  {#each look.rows as row (row.key)}
    <li>
      {#if row.day !== undefined}
        <p class="figure mt-snug py-tight text-note text-text-faint">{row.day}</p>
      {/if}
      {#if row.kind === "turn"}
        <div class="{ROW} mt-snug text-text">
          <span class={AT}>{row.at}</span>
          <span class="{KIND} font-label">{row.label}</span>
          <span class="{SUBJECT} text-text-quiet">{row.subject}</span>
          <span class={MEASURE}>{row.measure}</span>
        </div>
      {:else if row.kind === "call"}
        <button {...row.wire} class="{ROW} w-full text-left text-text-quiet hover:wash">
          <span class={AT}>{row.at}</span>
          <span class="{KIND} text-text-faint">{row.tool}</span>
          <span class={SUBJECT}>{row.subject}</span>
          <span class="{MEASURE} flex items-center gap-snug">
            {#if row.waiting}
              <span class="size-dot pulse rounded-pill bg-accent" aria-hidden="true"></span>
            {/if}
            {row.took}
            {#if row.exit !== undefined}
              <span class={row.exit.ok ? "text-accent" : "text-alert"}>{row.exit.said}</span>
            {/if}
            {#if row.outcome !== undefined}<span class={row.outcome.failed ? "text-alert" : ""}>{row.outcome.said}</span>{/if}
          </span>
        </button>
      {:else}
        <button {...row.wire} class={[ROW, "relative w-full text-left text-text", row.picked ? "here wash-strong" : "hover:wash"]}>
          <span class={AT}>{row.at}</span>
          <span class="{KIND} text-accent">{row.word}</span>
          <span class="{SUBJECT} figure">{row.short}</span>
          <span class={MEASURE}>
            {#if row.produced !== undefined}<Produced base={row.produced.base} head={row.produced.head} />{/if}
          </span>
        </button>
      {/if}
    </li>
  {/each}
</ol>

<style>
  /* The list fades out over its last 48 pixels at the density's scale,
   * so a row cut by the pane's foot reads as more to scroll to rather
   * than as the end. */
  .fade {
    mask-image: linear-gradient(to bottom, black calc(100% - var(--spacing-section) - var(--spacing-pane)), transparent);
  }

  /* The picked checkpoint carries the chosen bar down its leading edge,
   * standing in by one step from the row's top and bottom
   * (docs/frontend-method.md §7B). A border rather than a fill, so a
   * forced-colour mode keeps it. */
  .here::before {
    content: "";
    position: absolute;
    inset-block: var(--spacing-snug);
    inset-inline-start: 0;
    border-inline-start: var(--spacing-hair) solid var(--color-accent);
    border-radius: var(--radius-pill);
  }
</style>
