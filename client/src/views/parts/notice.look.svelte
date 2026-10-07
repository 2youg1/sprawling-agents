<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // How a notice is drawn (`./notice`, `NoticeLook`).
  //
  // Each seat carries the one entrance the motion vocabulary grants it:
  // a toast is a popover, and a popover rises into place; a strip in a
  // form and an entry in a list are state changes that keep their
  // position, so they fade. `theme/motion-state.css` stills every one of
  // these under the two reduction lists. There is deliberately no exit
  // class: a leaving notice is removed by whoever keeps the list, and
  // that owner animates the departure if it wants one at all.
  import type { Seat } from "./notice";

  const PAINT: Record<Seat, string> = {
    toast: "toast rise rounded-panel border border-edge-panel bg-raised px-pane py-base shadow-float",
    inline: "fade mt-tight rounded-card border border-edge-input px-snug py-tight",
    drawer: "fade w-full border-b border-edge px-base py-snug",
  };
</script>

<script lang="ts">
  import Badge from "./badge.svelte";
  import type { NoticeLook } from "./notice";

  const look: NoticeLook = $props();
</script>

<div {...look.region} class={[PAINT[look.seat], "flex min-w-0 flex-wrap items-start justify-between gap-base text-note"]}>
  <!-- The words keep a readable measure and the buttons move under them
  when the seat is narrower than both, rather than the words being
  squeezed to one character a line beside buttons that never shrink. -->
  <div class="flex min-w-0 grow basis-[16rem] flex-col gap-tight">
    <div class="flex min-w-0 flex-wrap items-baseline gap-snug">
      <span class={["min-w-0 font-label wrap-anywhere", look.weight === "alert" ? "text-alert" : "text-text"]}>
        {look.title}
      </span>
      {#if look.at !== undefined}
        <span class="shrink-0 text-text-faint">{look.at}</span>
      {/if}
      {#if look.count !== undefined}
        <Badge text={look.count} weight="quiet" />
      {/if}
      {#if look.code !== undefined}
        <span class="ml-auto shrink-0 font-mono text-note text-text-faint">{look.code}</span>
      {/if}
    </div>
    {#if look.next !== ""}
      <p class="min-w-0 wrap-anywhere text-note text-text">{look.next}</p>
    {/if}
    {#if look.fold !== undefined}
      <details class="min-w-0">
        <summary class="cursor-pointer text-note text-text-faint hover:text-text">{look.fold.summary}</summary>
        <div class="mt-tight flex min-w-0 flex-col gap-tight wrap-anywhere font-mono text-note text-text-quiet">
          {#each look.fold.lines as line, at (at)}
            <span>{line}</span>
          {/each}
        </div>
      </details>
    {/if}
  </div>
  {#if look.actions !== undefined}
    <div class="ml-auto flex shrink-0 items-center gap-tight">{@render look.actions()}</div>
  {/if}
</div>

<style>
  /* A toast keeps a readable measure and never runs past the viewport's
  inset on a narrow screen, nor past the stack or the box that holds it,
  which can be narrower than the viewport leaves. */
  .toast {
    width: min(var(--container-measure), calc(100vw - 2 * var(--spacing-pane)));
    max-width: 100%;
  }
</style>
