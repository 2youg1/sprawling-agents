<script lang="ts">
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// How an empty state is drawn, and nothing else (`./empty`,
// `EmptyLook`). Without a shape of the page's own it draws the empty
// ring, the glyph that already means "nothing in particular is
// happening", quiet enough that the sentence stays what is read.

import Glyph from "./glyph.svelte";
import type { EmptyLook, EmptySeat } from "./empty";

const look: EmptyLook = $props();

const SEATS: Record<EmptySeat, string> = {
  centred: "items-center px-pane py-section text-center",
  region: "items-start rounded-card border border-dashed border-edge-input px-wide py-wide text-left",
  inset: "items-start px-wide py-wide text-left",
};
</script>

<div class={["flex w-full flex-col gap-base", SEATS[look.seat]]}>
  <div {...look.shapeWire}>
    {#if look.shape !== undefined}
      {@render look.shape()}
    {:else}
      <Glyph name="ring" class="text-text-faint" />
    {/if}
  </div>
  <p class="max-w-measure text-note text-text-quiet">{look.sentence}</p>
  {#if look.action !== undefined}
    {@render look.action()}
  {/if}
</div>
