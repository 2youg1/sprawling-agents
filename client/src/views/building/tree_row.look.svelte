<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<!-- One row of the building's tree. A row is a row, so it takes the
row's wash under the pointer and the strong wash while it is the one in
the middle column (docs/frontend-method.md §7B); the chrome surface is
the right side's own. A folder's chevron turns toward open on the
arriving curve and back on the leaving one. -->
<script lang="ts">
  import Glyph from "../parts/glyph.svelte";
  import Tip from "../parts/tip.svelte";
  import type { TreeRowLook, TreeTone } from "./tree_row";

  const look: TreeRowLook = $props();

  const INK: Record<TreeTone, string> = {
    picked: "wash-strong text-text",
    hidden: "text-text-faint",
    plain: "text-text-quiet",
  };
</script>

<button
  {...look.wire}
  class={[
    "group/row flex h-step w-full items-center gap-tight rounded-control pl-tight pr-snug text-left text-note leading-none",
    look.tone === "picked" ? "" : "hover:wash",
    INK[look.tone],
  ]}
>
  <span class="flex w-glyph-sm shrink-0 justify-center">
    {#if look.mark.kind === "directory"}
      <Glyph
        name="chevron"
        size="sm"
        class={[
          "shrink-0 text-text-faint transition-transform still:transition-none",
          look.mark.open ? "rotate-90 ease-arrive" : "ease-leave",
        ]}
      />
    {:else if look.mark.kind === "transcript"}
      <Tip text={look.mark.hint}>
        {#snippet children(hint: string)}
          <!-- wording-ok: a typographic arrow, named by the hint it is labelled by -->
          <span class="text-text-faint" role="img" aria-labelledby={hint}>↗</span>
        {/snippet}
      </Tip>
    {/if}
  </span>
  <span class={["truncate", look.coded ? "font-mono text-text-faint" : ""]}>{look.name}</span>
  {#if look.live !== undefined}
    <Tip text={look.live}>
      {#snippet children(hint: string)}
        <span class="ml-tight inline-block size-dot shrink-0 rounded-pill bg-accent" role="img" aria-labelledby={hint}
        ></span>
      {/snippet}
    </Tip>
  {/if}
  <span class="flex-1"></span>
  {#if look.size !== undefined}
    <span class="hidden shrink-0 whitespace-nowrap font-mono text-text-faint group-hover/row:inline group-focus-visible/row:inline">
      {look.size}
    </span>
  {/if}
</button>
