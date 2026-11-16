<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The band of proposal cards above the document in RefRain
  // (client/Spec.lean §4-55): under its own rule, and scrolling by itself
  // once it would take more than two fifths of the right side, so a
  // long card never pushes the editor out. It takes `ProposalsLook`
  // (`./proposals`) and nothing else.
  import type { ProposalsLook } from "./proposals";

  const { band, heading, title, cards }: ProposalsLook = $props();
</script>

<section {...band} class="band">
  <h3 {...heading} class="text-note text-text-quiet">{title}</h3>
  {@render cards()}
</section>

<style>
  .band {
    display: flex;
    flex: none;
    flex-direction: column;
    gap: var(--spacing-snug);
    max-height: 40%;
    overflow-y: auto;
    padding: var(--spacing-snug) var(--spacing-wide);
    border-bottom: 1px solid var(--color-edge);
  }
</style>
