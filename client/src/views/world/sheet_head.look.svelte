<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- How the head of the world's sheet is drawn: a bar-high back key at
the top of the side the sheet came from, then the strip of tabs on a
hairline, the shown tab underlined in the accent. On a phone the names
outrun the strip; the shown tab keeps its whole name and the others give
up their ends, so the name of what the sheet shows is never the one
cut. Every role, key and `aria-*` value comes in the bags. -->
<script lang="ts">
  import Glyph from "../parts/glyph.svelte";
  import type { SheetHeadLook } from "./sheet_head";

  const look: SheetHeadLook = $props();
</script>

<div class="flex min-w-0 items-center gap-snug border-b border-edge">
  <button {...look.back} class="-ml-snug grid size-bar shrink-0 place-items-center rounded-control text-text-quiet hover:wash hover:text-text">
    <Glyph name="chevron" class="rotate-180" />
  </button>
  <div {...look.strip} class="flex min-w-0 items-stretch overflow-x-auto">
    {#each look.tabs as tab (tab.key)}
      <button
        {...tab.wire}
        class={[
          "flex h-bar min-w-0 items-center border-b-2 px-base text-label whitespace-nowrap transition-colors",
          tab.shown ? "shrink-0 border-accent text-text ease-arrive" : "border-transparent text-text-quiet ease-leave hover:text-text hover:ease-arrive",
        ]}
      >
        <span class="truncate">{tab.label}</span>
      </button>
    {/each}
  </div>
</div>
