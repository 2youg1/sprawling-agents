<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // One settings card, the shipped look: markup only. Every word and
  // the save's standing arrive as `CardLook` from the seat
  // (`card.svelte`). The standing is a live region, so the receipt is
  // announced when it arrives rather than when the button is pressed.

  import Button from "../parts/button.svelte";
  import Glyph from "../parts/glyph.svelte";
  import type { CardLook } from "./card";

  const { title, note, standing, press, children }: CardLook = $props();
</script>

<div class="flex min-w-0 flex-col gap-snug rounded-card bg-raised px-base py-snug">
  <div class="flex flex-col gap-hair">
    <span class="text-label font-label text-text">{title}</span>
    <p class="text-note text-text-faint">{note}</p>
  </div>
  <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
  {@render children()}
  <div class="mt-tight flex items-center gap-base">
    <span class="flex min-w-0 flex-1 flex-col gap-hair text-note" role="status">
      {#if standing.word !== null}
        <span class={["inline-flex items-center gap-tight", standing.weight === "alert" ? "text-alert" : "text-text-quiet"]}>
          {#if standing.saved}
            <Glyph name="check" size="sm" class="shrink-0" />
          {/if}
          {standing.word}
        </span>
      {/if}
      {#if standing.detail !== null}
        <span class="text-text-faint">{standing.detail}</span>
      {/if}
    </span>
    {#if press !== null}
      <Button
        label={press.label}
        tone="primary"
        loading={press.loading}
        {...(press.why === undefined ? {} : { why: press.why })}
        onPress={press.onPress}
      />
    {/if}
  </div>
</div>
