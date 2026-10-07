<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // How the monitor is drawn, and nothing else (client D95): a header
  // with the run's file count and two toggles, then the code pane with
  // the terminal or the folded column of files beside it. Every word
  // arrives translated and every role, `aria-*` value, handler and pane
  // attachment arrives inside a wire bag (`./monitor.ts`), spread
  // unchanged on the element it is for, so another look draws the same
  // monitor by taking the same `MonitorLook`.
  //
  // Each control's colour arrives decelerating and leaves accelerating
  // (docs/frontend-method.md §4-43); the durations are theme tokens, so
  // motion off stills them with no rule here.
  import type { MonitorLook } from "./monitor";

  const look: MonitorLook = $props();

  const TOGGLE = "inline-flex h-control-sm items-center rounded-control px-snug text-label transition-colors ease-leave hover:ease-arrive";
  const PANE = "pane relative min-h-0 flex-1 overflow-y-auto focus-visible:outline-2 focus-visible:outline-accent";
</script>

<section class="flex min-h-0 flex-1 flex-col" aria-label={look.label}>
  <header class="flex flex-wrap items-center gap-snug border-b border-edge bg-chrome px-pane py-snug text-note">
    <span class="text-label text-text">{look.heading}</span>
    <span class="text-text-faint figure">{look.tally}</span>
    <!-- Following is the resting state and reads as a lit word; paused
    stands on a raised fill, because it is the state that asks to be
    taken back up. -->
    <button
      class="{TOGGLE} ms-auto {look.follow.held ? 'text-accent hover:bg-raised' : 'bg-raised text-text hover:bg-raised-hover'}"
      {...look.follow.wire}>{look.follow.label}</button
    >
    <button class="{TOGGLE} text-text-quiet hover:bg-raised hover:text-text" {...look.fold.wire}>{look.fold.label}</button>
  </header>
  <div class="flex min-h-0 flex-1">
    {#if look.index !== undefined}
      <nav class="w-index shrink-0 overflow-y-auto border-e border-edge py-tight" aria-label={look.index.label}>
        {#each look.index.files as file (file.path)}
          <button
            class="block w-full truncate px-snug py-tight text-start font-mono text-note text-text-quiet transition-colors ease-leave hover:bg-raised hover:text-text hover:ease-arrive"
            {...file.wire}>{file.path}</button
          >
        {/each}
      </nav>
    {/if}
    <div class="{PANE} {look.record === undefined ? '' : 'beside border-e border-edge'}" {...look.code}>
      {@render look.column()}
    </div>
    {#if look.record !== undefined}
      <div class="{PANE} bg-page" {...look.record}>
        {@render look.printed()}
      </div>
    {/if}
  </div>
</section>

<style>
  /* The code takes the larger share beside the terminal, because a
  hunk's lines are wider than a command's output. */
  .pane.beside {
    flex-grow: 1.4;
  }
</style>
