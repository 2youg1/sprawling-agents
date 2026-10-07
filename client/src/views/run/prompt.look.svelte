<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<!-- How the prompt lens is drawn, and nothing else: every word arrives
translated and every press decided (`./prompt`, `PromptLook`). A segment
is a row that opens in place, so it takes the wash under the pointer the
way a row does, and its chevron turns toward open on the arriving curve
and back on the leaving one (docs/frontend-method.md §4-43); the copy key
is a control, so it lifts from raised to raised-hover. -->
<script lang="ts">
  import Glyph from "../parts/glyph.svelte";
  import Path from "../parts/path.svelte";
  import type { PromptLook, SegmentLook } from "./prompt";

  const look: PromptLook = $props();
</script>

{#snippet segment(each: SegmentLook)}
  <li class="border-b border-edge">
    <div class="flex items-center gap-base py-tight text-note">
      <button
        {...each.fold}
        class={[
          "flex min-h-control-sm min-w-0 flex-1 items-center gap-base rounded-control px-snug text-left transition-colors still:transition-none",
          "ease-leave hover:wash hover:ease-arrive",
        ]}
      >
        <Glyph
          name="chevron"
          size="sm"
          class={[
            "shrink-0 text-text-faint transition-transform still:transition-none",
            each.open ? "rotate-90 ease-arrive" : "ease-leave",
          ]}
        />
        <span class="shrink-0 text-text-quiet">{each.slot}</span>
        <span class="figure shrink-0 text-text-faint">{each.size}</span>
        {#if each.gone !== undefined}
          <span class="truncate text-alert">{each.gone}</span>
        {/if}
      </button>
      {#if each.copy !== undefined}
        <button
          {...each.copy.wire}
          class={[
            "inline-flex h-control-sm shrink-0 items-center gap-tight rounded-control bg-raised px-snug text-label text-text-quiet transition-colors still:transition-none",
            "ease-leave hover:bg-raised-hover hover:text-text hover:ease-arrive",
          ]}
        >
          {#if each.copy.copied}
            <Glyph name="check" size="sm" />
          {/if}
          {each.copy.label}
        </button>
      {/if}
    </div>
    {#if each.open}
      <div class="flex flex-col gap-snug pb-base pl-wide">
        {#if each.sources.length > 0}
          <p class="flex flex-wrap items-baseline gap-snug text-note text-text-faint">
            <span>{each.sourcesLabel}</span>
            {#each each.sources as source (source.key)}
              <Path path={source.path} onOpen={source.onOpen} />
              {#if source.dropped !== undefined}
                <span class="text-alert">{source.dropped}</span>
              {/if}
            {/each}
          </p>
        {/if}
        <pre
          class="overflow-x-auto whitespace-pre-wrap break-words rounded-control border border-edge bg-page p-base font-mono text-note text-text-quiet">{each.text}</pre>
      </div>
    {/if}
  </li>
{/snippet}

{#if look.segments === undefined}
  <p class="text-text-faint">…</p>
{:else if look.segments.length > 0}
  <ul>
    {#each look.segments as each (each.key)}
      <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
      {@render segment(each)}
    {/each}
  </ul>
{:else}
  <p class="text-text-faint">{look.none}</p>
{/if}
