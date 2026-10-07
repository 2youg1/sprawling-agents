<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The line above a PDF or a DOCX that picks the version it is compared
  // with: a native list, because a version's name fits no equal cell.
  // It takes `AgainstLook` (`./against`) and nothing else.
  import type { AgainstLook } from "./against";

  const { label, none, options, wire }: AgainstLook = $props();
</script>

<div class="flex items-center gap-base border-b border-edge px-wide py-tight">
  <label class="flex min-w-0 flex-1 items-center gap-snug text-note text-text-faint">
    {label}
    <select
      class="h-control-sm min-w-0 flex-1 rounded-control border border-edge-input bg-raised px-snug text-note text-text"
      {...wire}
    >
      <option value="">{none}</option>
      {#each options as option (option.value)}
        <option value={option.value}>{option.label}</option>
      {/each}
    </select>
  </label>
</div>
