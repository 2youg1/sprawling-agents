<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts">
  // How the microphone is drawn, and nothing else: a round key that
  // shows the mic mark (never the word "speak", which a person reading
  // for the first time has to guess), turns the alert colour while it
  // records and goes faint while the city transcribes (`./record.ts`,
  // `RecordLook`). Hover lifts the wash, a press yields by the theme's
  // press scale only while the key can be used, release returns on the
  // leave curve (§4-43). Its hit area reaches past the key, because the
  // drawn square is shorter than a fingertip. The action name rides in a
  // `Tip`, so focus and touch reach it the way other icon keys do.
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
    /* Reach the theme's touch square (§4-34), centred on the drawn key,
     * so a compact density does not shrink the hit area below 44 px. */
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
