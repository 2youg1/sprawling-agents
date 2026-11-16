<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The column a person picks a building in. What stands in it, and in
  // what order, is `buildings.ts`'s decision; this file only draws it.
  import type { BuildingColumnProps } from "./buildings";
  import { HALL } from "./buildings";

  const { label, buildings, chosen, onPick, hall }: BuildingColumnProps = $props();
</script>

<nav class="w-tree max-w-full shrink-0" aria-label={label}>
  <ul class="flex flex-col gap-tight">
    {#each buildings as addr (addr)}
      <li>
        <button
          type="button"
          class={[
            "h-control-sm w-full rounded-control px-base text-left text-note",
            chosen === addr ? "bg-raised text-text" : "text-text-quiet hover:bg-chrome",
          ]}
          aria-current={chosen === addr ? "true" : undefined}
          onclick={() => {
            onPick(addr);
          }}
        >
          {addr === HALL ? hall : addr}
        </button>
      </li>
    {/each}
  </ul>
</nav>
