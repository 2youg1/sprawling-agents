<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<!-- The sandbox's choice of arm: the platform's own menu, on the surface
and edge every input wears, at the height of a control. -->
<script lang="ts">
  import type { ArmSelectLook } from "./arm_select";

  const look: ArmSelectLook = $props();
</script>

<select
  {...look.wire}
  class="h-control w-full rounded-control border border-edge-input bg-raised px-base text-note text-text"
>
  {#each look.options as option (option.value)}
    <option value={option.value}>{option.label}</option>
  {/each}
</select>
