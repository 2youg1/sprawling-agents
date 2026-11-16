<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<!-- How the archive's hits are drawn (`./archive`, `ArchiveLook`): a row
per hit on shared column lines, the building as the link. The link
lights on the arriving curve and settles on the leaving one
(docs/frontend-method.md §4-43). -->
<script lang="ts">
  import type { ArchiveLook } from "./archive";

  const look: ArchiveLook = $props();
</script>

<ul class="text-note">
  {#each look.hits as hit (hit.key)}
    <li class="settled-row flex items-baseline gap-base border-b border-edge py-snug">
      <span class="figure w-figure shrink-0 text-text-faint">{hit.day}</span>
      <span class="w-figure shrink-0 truncate text-text-faint">{hit.kind}</span>
      <a
        {...hit.wire}
        class="block max-w-[24ch] shrink-0 truncate font-mono text-text-quiet transition-colors ease-leave still:transition-none hover:text-text hover:ease-arrive"
      >
        {hit.building}
      </a>
      <span class="flex-1 truncate text-text">{hit.subject}</span>
    </li>
  {/each}
</ul>
