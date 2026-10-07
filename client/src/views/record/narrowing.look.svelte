<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<!-- How one narrowing choice is drawn (`./narrowing`, `NarrowingLook`):
its name, then the browser's own select, which keeps the platform's
keyboard and its list on a phone. It is a control, so it lifts from
raised to raised-hover on the arriving curve (docs/frontend-method.md
§4-43). -->
<script lang="ts">
  import type { NarrowingLook } from "./narrowing";

  const look: NarrowingLook = $props();
</script>

<label class="flex items-center gap-snug text-note text-text-faint">
  {look.label}
  <select
    {...look.wire}
    class={[
      "h-control-sm max-w-[24ch] rounded-control bg-raised px-snug font-mono text-note text-text transition-colors still:transition-none",
      "ease-leave hover:bg-raised-hover hover:ease-arrive",
    ]}
  >
    {#each look.options as option (option.value)}
      <option value={option.value}>{option.label}</option>
    {/each}
  </select>
</label>
