<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // How the export buttons are drawn (`./usage_export`,
  // `UsageExportLook`): two quiet buttons in a row, and under them the
  // city's reason when it could not answer the last press.
  import Button from "./button.svelte";
  import type { UsageExportLook } from "./usage_export";

  const look: UsageExportLook = $props();
</script>

<div class="flex flex-wrap items-center gap-snug">
  {#each look.buttons as button (button.format)}
    <Button label={button.label} tone="quiet" loading={button.loading} onPress={button.press} />
  {/each}
  {#if look.unavailable !== undefined}
    <p class="flex basis-full flex-col gap-tight text-note text-text-faint wrap-anywhere">
      <span>{look.unavailable.said}</span>
      {#if look.unavailable.reason !== undefined}<span class="font-mono">{look.unavailable.reason}</span>{/if}
    </p>
  {/if}
</div>
