<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // How a badge is drawn (`./badge`, `BadgeLook`). A word badge is a
  // pill in a line of text, its tier in the ink; a count is a solid
  // pill one key-glyph high, as narrow as a circle until its number
  // needs more, and a lone dot when there is no number. Where a count
  // stands is its caller's layout: this draws the mark, not its corner.
  import type { Weight } from "./glyph";

  const INK: Record<Weight, string> = {
    quiet: "bg-raised text-text-quiet",
    live: "bg-raised text-accent",
    alert: "bg-raised text-alert",
  };

  const FILL: Record<Weight, string> = {
    quiet: "bg-mark",
    live: "bg-accent",
    alert: "bg-alert",
  };

  const SOLID: Record<Weight, string> = {
    quiet: "bg-raised text-text",
    live: "bg-accent text-on-accent",
    alert: "bg-alert text-on-accent",
  };
</script>

<script lang="ts">
  import Glyph from "./glyph.svelte";
  import type { BadgeLook } from "./badge";

  const look: BadgeLook = $props();
</script>

{#if look.form === "word"}
  <span class={["inline-flex items-center gap-tight rounded-pill px-snug py-tight text-note whitespace-nowrap", INK[look.tier]]}>
    {#if look.mark === "dot"}
      <span class={["inline-block size-dot rounded-pill", FILL[look.tier]]} aria-hidden="true"></span>
    {:else if look.mark !== undefined}
      <Glyph name={look.mark} size="sm" class="shrink-0" />
    {/if}
    {look.text}
  </span>
{:else if look.text === undefined}
  <span class={["inline-block size-dot rounded-pill", FILL[look.tier]]} {...look.quiet}></span>
{:else}
  <span
    class={[
      "inline-flex h-glyph-key min-w-glyph-key items-center justify-center rounded-pill px-tight",
      "text-tally font-label leading-none whitespace-nowrap",
      SOLID[look.tier],
    ]}
    {...look.quiet}>{look.text}</span
  >
{/if}
