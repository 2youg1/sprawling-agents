<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  import type { MonitorLook } from "./monitor";

  const look: MonitorLook = $props();

  const TOGGLE =
    "inline-flex h-control-sm items-center rounded-control px-snug text-label transition-colors ease-leave hover:ease-arrive";
  const PANE =
    "pane relative min-h-0 flex-1 overflow-y-auto focus-visible:outline-2 focus-visible:outline-accent";
</script>

<section class="flex min-h-0 flex-1 flex-col" aria-label={look.label}>
  <header
    class="flex flex-wrap items-center gap-snug border-b border-edge bg-chrome px-pane py-snug text-note"
  >
    <span class="text-label text-text">{look.heading}</span>
    <span class="text-text-faint figure">{look.tally}</span>
    <button
      class="{TOGGLE} ms-auto {look.follow.held
        ? 'text-accent hover:bg-raised'
        : 'bg-raised text-text hover:bg-raised-hover'}"
      {...look.follow.wire}>{look.follow.label}</button
    >
    <button class="{TOGGLE} text-text-quiet hover:bg-raised hover:text-text" {...look.fold.wire}
      >{look.fold.label}</button
    >
  </header>
  <div class="flex min-h-0 flex-1">
    {#if look.index !== undefined}
      <nav
        class="w-index shrink-0 overflow-y-auto border-e border-edge py-tight"
        aria-label={look.index.label}
      >
        {#each look.index.files as file (file.path)}
          <button
            class="block w-full truncate px-snug py-tight text-start font-mono text-note text-text-quiet transition-colors ease-leave hover:bg-raised hover:text-text hover:ease-arrive"
            {...file.wire}>{file.path}</button
          >
        {/each}
      </nav>
    {/if}
    <div
      class="{PANE} {look.record === undefined ? '' : 'beside border-e border-edge'}"
      {...look.code}
    >
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
  .pane.beside {
    flex-grow: 1.4;
  }
</style>
