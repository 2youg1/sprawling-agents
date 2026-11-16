<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts">
  import Glyph from "../parts/glyph.svelte";
  import Tip from "../parts/tip.svelte";
  import type { RecordLook } from "./record";

  const look: RecordLook = $props();
</script>

<Tip text={look.label}>
  {#snippet children(_hint: string)}
    <button
      class="microphone transition-[background-color,color,scale] ease-leave hover:ease-arrive aria-[disabled=false]:active:scale-(--scale-press) aria-[disabled=false]:active:ease-arrive still:transition-none still:aria-[disabled=false]:active:scale-100"
      data-hearing={look.hearing}
      {...look.wire}
    >
      <Glyph name="mic" size="sm" />
    </button>
  {/snippet}
</Tip>
{#if look.refused !== undefined}
  <span class="refused">{look.refused}</span>
{/if}

<style>
  .microphone {
    position: relative;
    display: grid;
    flex-shrink: 0;
    place-items: center;
    width: var(--spacing-control-sm);
    height: var(--spacing-control-sm);
    border-radius: var(--radius-pill);
    background-color: var(--color-raised);
    color: var(--color-text-quiet);
  }
  .microphone::before {
    content: "";
    position: absolute;
    inset: calc((var(--spacing-touch) - var(--spacing-control-sm)) / -2);
  }
  .microphone[data-hearing="ready"]:hover {
    background-color: var(--color-raised-hover);
    color: var(--color-text);
  }
  .microphone[data-hearing="taking"] {
    background-color: var(--color-alert);
    color: var(--color-on-accent);
  }
  .microphone[data-hearing="taking"]:hover {
    filter: brightness(1.06);
  }
  .microphone[data-hearing="hearing"] {
    color: var(--color-text-disabled);
  }
  .refused {
    color: var(--color-alert);
    font-size: var(--text-note);
  }
</style>
