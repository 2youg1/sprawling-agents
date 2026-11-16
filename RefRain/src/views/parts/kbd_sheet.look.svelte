<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The sheet every chord is read on, drawn in the same box as the
  // irreversible question: a heading with the way out beside it, one row
  // per action with its chord at the row's end, and where the chords
  // are rebound.
  import Button from "./button.svelte";
  import Kbd from "./kbd.look.svelte";
  import Sheet from "./sheet.look.svelte";
  import type { CheatsheetLook } from "./kbd";

  const { sheet, stands, heading, dismiss, rows, where }: CheatsheetLook = $props();
</script>

<Sheet wire={sheet} {stands}>
  <div class="flex items-baseline justify-between">
    <h2 id={heading.id} class="text-heading font-heading text-text">{heading.text}</h2>
    <Button label={dismiss.label} tone="quiet" onPress={dismiss.onPress} />
  </div>
  <ul class="flex flex-col">
    {#each rows as row (row.key)}
      <li class="flex items-center justify-between gap-base py-tight text-label">
        <span class="truncate text-text-quiet">{row.label}</span>
        <Kbd marks={row.marks} />
      </li>
    {/each}
  </ul>
  <p id={where.id} class="text-note text-text-faint">{where.text}</p>
</Sheet>
