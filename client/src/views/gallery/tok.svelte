<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts" module>
  // The surfaces client/Spec.lean §4-43 names, each in the states a person can
  // put it in: glass drawn and glass turned solid, over the same words so
  // the difference is what shows through; every rounded box at the one
  // corner exponent; and the appearance group, where glass and the blend
  // tier's opacity are chosen.
  import type { GlyphName } from "../parts/glyph";

  const KEYS: readonly GlyphName[] = ["layers", "inbox", "settings"];

  // What stands behind the keys: a page of words, which is the hardest
  // thing glass has to keep apart from what it carries.
  const BEHIND = Array.from({ length: 9 }, (_, n) => `crates/city/src/document.rs:${String(40 + n * 7)} reads the version the person saw`);

  const CORNERS: readonly { readonly box: string; readonly size: string }[] = [
    { box: "rounded-panel", size: "h-[96px] w-[160px]" },
    { box: "rounded-card", size: "h-[64px] w-[120px]" },
    { box: "rounded-control", size: "h-control w-[96px]" },
    { box: "rounded-key", size: "size-key" },
    { box: "rounded-pill", size: "h-control-sm w-[96px]" },
  ];
</script>

<script lang="ts">
  import { EDGE_KEY } from "../edge.svelte";
  import Setup from "../setup.svelte";
  import Glyph from "../parts/glyph.svelte";
  import Case from "./case.svelte";
  import { ENDPOINTS } from "./served";
</script>

{#snippet keysOver(glass: "on" | "off")}
  <div class="relative h-[200px] overflow-hidden bg-page" data-glass={glass}>
    <div class="flex flex-col gap-tight p-snug font-mono text-note text-text-quiet" aria-hidden="true">
      {#each BEHIND as line (line)}
        <span>{line}</span>
      {/each}
    </div>
    <div class="absolute bottom-snug left-wide flex flex-col gap-snug">
      {#each KEYS as key (key)}
        <span class={EDGE_KEY}><Glyph name={key} size="key" /></span>
      {/each}
    </div>
  </div>
{/snippet}

<Case label="surfaces · the edge keys as glass over a page of words" width={360}>
  <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
  {@render keysOver("on")}
</Case>

<Case label="surfaces · the edge keys with glass turned solid" width={360}>
  <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
  {@render keysOver("off")}
</Case>

<Case label="surfaces · every rounded box at the one corner exponent" width={760}>
  <div class="flex flex-wrap items-end gap-wide">
    {#each CORNERS as corner (corner.box)}
      <span class="{corner.box} {corner.size} shrink-0 border border-edge-panel bg-raised"></span>
    {/each}
  </div>
</Case>

<Case label="surfaces · the appearance group, where glass and the blend opacity are chosen" width={1280}>
  <Setup
    group="appearance"
    endpoints={ENDPOINTS}
    autonomy={undefined}
    onAutonomy={() => undefined}
  />
</Case>
