<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<!-- A row that opens in place. The chevron in the first column says the
row opens before a hand finds out by pressing it, and it turns with the
row: toward open on the arriving curve, back on the leaving one
(docs/frontend-method.md §4-43). The row takes the wash under the
pointer, the way every row does; a control's lift is for controls. -->
<script lang="ts">
  import Glyph from "../parts/glyph.svelte";
  import type { DisclosureLayout, DisclosureLook } from "./disclosure";

  const look: DisclosureLook = $props();

  const COLUMNS: Record<DisclosureLayout, string> = {
    plan: "min-h-control grid-cols-[var(--spacing-glyph-sm)_minmax(0,1fr)_auto] py-tight",
    commit:
      "h-control grid-cols-[var(--spacing-glyph-sm)_8ch_minmax(0,1fr)_auto_auto] narrow:grid-cols-[var(--spacing-glyph-sm)_8ch_minmax(0,1fr)]",
    change: "min-h-control grid-cols-[var(--spacing-glyph-sm)_var(--spacing-figure)_minmax(0,1fr)_auto] py-tight",
  };
</script>

<button
  {...look.wire}
  class="grid w-full items-center gap-x-base rounded-control px-snug text-left text-note hover:wash {COLUMNS[look.layout]}"
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
