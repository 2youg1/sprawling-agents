<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<!-- How the recycle bin is drawn (`./bin`, `BinLook`): a row per
discard, what and when on the first line with whether it is back, and
the way back under it. The press that restores is the client's one
button, in its quiet tone, at the end of the line it acts on. A row is
two lines, so it is not a `settled-row`, whose deferral reserves one
step and lets the second line spill out of the row until it is in view. -->
<script lang="ts">
  import Button from "../parts/button.svelte";
  import Path from "../parts/path.svelte";
  import type { BinLook, Standing } from "./bin";

  const look: BinLook = $props();

  const INK: Record<Standing, string> = {
    restored: "text-text-faint",
    gone: "text-alert",
  };
</script>

<ul class="text-note">
  {#each look.rows as row (row.key)}
    <li class="flex flex-col gap-tight border-b border-edge py-snug">
      <div class="flex min-h-control items-center gap-base">
        <span class="min-w-0 flex-1">
          <Path path={row.path} />
        </span>
        <span class="figure text-text-faint">{row.when}</span>
        <span class={INK[row.standing]}>{row.standingWord}</span>
        {#if row.restore !== undefined}
          <Button label={row.restore.label} tone="quiet" onPress={row.restore.onPress} />
        {/if}
      </div>
      <div class="summary truncate font-mono text-text-faint">{row.way}</div>
    </li>
  {/each}
</ul>
