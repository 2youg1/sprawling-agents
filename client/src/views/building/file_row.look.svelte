<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<!-- One changed file of the working tree. The row whose file is in front
on the right side is marked the way a chosen row is marked everywhere -
the strong wash and the 2 px accent bar on its left
(docs/frontend-method.md §7B); any other row takes the row's wash under
the pointer. The bar stands on the row rather than in the button, so it
spans the row's height beside whatever control follows. -->
<script lang="ts">
  import type { FileRowLook } from "./file_row";

  const look: FileRowLook = $props();
</script>

<div class="relative flex min-w-0 flex-1">
  {#if look.current}
    <span class="absolute inset-y-snug left-0 w-hair rounded-pill bg-accent" aria-hidden="true"></span>
  {/if}
  <button
    {...look.wire}
    class={[
      "grid h-control min-w-0 flex-1 grid-cols-[16ch_minmax(0,1fr)_auto] items-center gap-x-base rounded-control pr-snug pl-base text-left narrow:grid-cols-[8ch_minmax(0,1fr)_auto]",
      look.current ? "wash-strong" : "hover:wash",
    ]}
  >
    <span class="truncate text-text-faint">{look.how}</span>
    <span class="truncate font-mono text-text">{look.path}</span>
    <span class="figure text-text-faint">{look.lines}</span>
  </button>
</div>
