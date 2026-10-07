<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // How a building's desktop allowlist editor is drawn, and nothing else
  // (client D95): one box holding the whole file, the save, and where
  // the file lives. Every word arrives translated in `DesktopLook`
  // (`./desktop.ts`); the box's bag is spread unchanged on the box.
  import Button from "./parts/button.svelte";
  import type { DesktopLook } from "./desktop";

  const look: DesktopLook = $props();
</script>

<div class="flex flex-col gap-snug">
  <textarea
    class="min-h-output w-full rounded-control border border-edge-input bg-raised px-base py-snug font-mono text-note text-text placeholder:text-text-faint"
    {...look.box}
  ></textarea>
  {#if look.missing !== undefined}
    <p class="text-note text-text-faint">{look.missing}</p>
  {/if}
  <div class="flex items-center gap-base">
    <Button label={look.save.label} tone="primary" {...(look.save.why === undefined ? {} : { why: look.save.why })} onPress={look.save.press} />
    {#if look.none !== undefined}
      <span class="text-note text-text-faint">{look.none}</span>
    {/if}
    <span class="flex-1"></span>
    <code class="truncate font-mono text-note text-text-faint">{look.scope}</code>
  </div>
</div>
