<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  import Glyph from "../parts/glyph.svelte";
  import type { DisclosureLayout, DisclosureLook } from "./disclosure";
  const look: DisclosureLook = $props();

  const COLUMNS: Record<DisclosureLayout, string> = {
    plan: "min-h-control grid-cols-[var(--spacing-glyph-sm)_minmax(0,1fr)_auto] gap-x-snug py-tight pr-snug",
    commit:
      "h-control grid-cols-[var(--spacing-glyph-sm)_8ch_minmax(0,1fr)_auto_auto] gap-x-base px-snug narrow:grid-cols-[var(--spacing-glyph-sm)_8ch_minmax(0,1fr)]",
    change:
      "min-h-control grid-cols-[var(--spacing-glyph-sm)_var(--spacing-figure)_minmax(0,1fr)_auto] gap-x-base py-tight px-snug",
  };
</script>

<button
  {...look.wire}
  class="grid w-full items-center rounded-control text-left text-note hover:wash {COLUMNS[
    look.layout
  ]}"
>
  <Glyph
    name="chevron"
    size="sm"
    class={[
      "text-text-faint transition-transform still:transition-none",
      look.open ? "rotate-90 ease-arrive" : "ease-leave",
    ]}
  />
  {@render look.cells()}
</button>
