<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  import type { SectionsLook } from "./sections";
  const look: SectionsLook = $props();
</script>

<ul class="flex flex-col narrow:flex-row narrow:flex-wrap narrow:gap-tight">
  {#each look.entries as entry (entry.key)}
    <li>
      <button
        {...entry.wire}
        class={[
          "relative flex h-control w-full items-center rounded-control px-snug text-left text-note narrow:w-auto",
          entry.current ? "wash-strong text-text" : "text-text-quiet hover:wash",
        ]}
      >
        {#if entry.current}
          <span
            class="absolute inset-y-snug left-0 w-hair rounded-pill bg-accent"
            aria-hidden="true"
          ></span>
        {/if}
        {entry.label}
      </button>
    </li>
  {/each}
</ul>
