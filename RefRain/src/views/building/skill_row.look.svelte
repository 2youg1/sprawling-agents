<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  import type { SkillRowLook } from "./skill_row";
  const look: SkillRowLook = $props();
</script>

<button
  {...look.wire}
  class="flex w-full min-w-0 flex-col gap-tight rounded-control px-snug py-snug text-left text-note enabled:hover:wash disabled:cursor-default"
>
  <span class="flex w-full min-w-0 items-baseline gap-base">
    <span class="shrink-0 font-mono text-text-quiet">{look.name}</span>
    <span class="min-w-0 flex-1 truncate text-text-faint">{look.summary}</span>
  </span>
  <span class="flex w-full min-w-0 items-baseline gap-base">
    <span class="shrink-0 text-text-faint">{look.shelf}</span>
    <span class="min-w-0 flex-1 truncate font-mono text-text-faint">{look.at}</span>
    <span class="min-w-0 truncate text-text-faint">{look.admitted}</span>
    <span class="min-w-0 truncate text-text-faint">{look.used}</span>
    {#if look.changed !== undefined}
      <span class="min-w-0 truncate text-text-quiet">{look.changed}</span>
    {/if}
  </span>
</button>
