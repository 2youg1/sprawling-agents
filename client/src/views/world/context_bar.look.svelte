<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // How a sessions row's context bar is drawn, and nothing else: a hair
  // of track, the used share filled from the left, and the handoff
  // checkpoint drawn by the same look the context ring uses. The share
  // in words, already said, is for a screen reader; the bar is drawing.
  import Checkpoint from "../talk/checkpoint.look.svelte";
  import type { ContextBarLook } from "../talk/gauge";

  const { share, handoff, said }: ContextBarLook = $props();
</script>

<span class="relative block h-hair rounded-pill bg-edge-panel" aria-hidden="true">
  <span class="used absolute inset-y-0 left-0 rounded-pill bg-text-quiet" style:width="{share}%"></span>
  {#if handoff !== undefined}
    <Checkpoint {...handoff} along="line" />
  {/if}
</span>
<span class="sr-only">{said}</span>

<style>
  .used {
    transition: width var(--transition-duration-page) var(--ease-arrive);
  }
</style>
