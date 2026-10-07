<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- How the signals queued for this room are drawn (`InboxLook`,
`./inbox`): a label-weight heading over one row per signal - its kind,
its first line taking the room that is left, who sent it held to under a
third of the row, and how long ago - every row a link to the sender's
room, lit while a pointer rests on it. -->
<script lang="ts">
  import type { InboxLook } from "./inbox";

  const { label, heading, rows }: InboxLook = $props();
</script>

<section class="text-note" aria-label={label}>
  <h2 class="text-label font-label text-text-quiet">{heading}</h2>
  <ul>
    {#each rows as row (row.key)}
      <li>
        <a href={row.href} class="flex items-baseline gap-base rounded-control py-tight hover:bg-raised-hover">
          <span class="shrink-0 text-text-quiet">{row.kind}</span>
          <span class="min-w-0 flex-1 truncate">{row.first}</span>
          <span class="max-w-3/10 truncate text-text-faint">{row.from}</span>
          <span class="shrink-0 text-text-faint">{row.ago}</span>
        </a>
      </li>
    {/each}
  </ul>
</section>
