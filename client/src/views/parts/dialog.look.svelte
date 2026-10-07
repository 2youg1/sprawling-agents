<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The irreversible question, drawn: a heading, what it will cost, and
  // the two answers on one row at the end, in the order they are handed.
  import Button from "./button.svelte";
  import Sheet from "./sheet.look.svelte";
  import type { DialogLook } from "./dialog";

  const { sheet, stands, heading, detail, answers }: DialogLook = $props();
</script>

<Sheet wire={sheet} {stands}>
  <h2 id={heading.id} class="text-heading text-text">{heading.text}</h2>
  {#if detail !== undefined}
    <p id={detail.id} class="text-note text-text-quiet">{detail.text}</p>
  {/if}
  <div class="flex items-center justify-end gap-snug">
    {#each answers as answer (answer.tone)}
      <Button label={answer.label} tone={answer.tone} onPress={answer.onPress} />
    {/each}
  </div>
</Sheet>
