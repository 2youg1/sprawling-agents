<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // How the head of one step of the guide is drawn, and nothing else
  // (`./step.ts`, `StepLook`): a row across the column - the number in
  // figures, the title, and at the end where the step stands, in the ink
  // that standing takes. The head is a row of a list, so it takes the
  // ink wash under the pointer and while it holds the focus
  // (docs/frontend-method.md §7A-5), arriving on the arriving curve and
  // leaving on the leaving one (§4-43).
  import Glyph from "../parts/glyph.svelte";
  import type { Standing } from "./guide";
  import type { StepLook } from "./step";

  const look: StepLook = $props();

  const INK: Readonly<Record<Standing, string>> = {
    configured: "text-text",
    required: "text-alert",
    skipped: "text-text-faint",
    seen: "text-text-quiet",
    untouched: "text-text-faint",
  };
</script>

<h2>
  <button
    {...look.wire}
    class="grid w-full grid-cols-[4ch_minmax(0,1fr)_auto] items-baseline gap-x-base py-base text-left transition-colors ease-leave hover:wash hover:ease-arrive focus-visible:wash"
  >
    <span class="figure text-heading text-text-faint">{look.number}</span>
    <span class="min-w-0 text-label font-label text-text">{look.title}</span>
    <span class={["flex items-center gap-tight text-note", INK[look.standing]]}>
      {#if look.standing === "configured"}
        <Glyph name="check" class="size-glyph-sm" />
      {/if}
      {#if look.word !== undefined}{look.word}{/if}
    </span>
  </button>
</h2>
