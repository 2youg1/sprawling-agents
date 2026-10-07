<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- How the commits pane is drawn: one row per commit, newest first - its
piece of the swimlane graph, the short oid, the message and the room
that made it. The chosen session's lane and its nodes take the accent
(docs/frontend-method.md §7B), a node the colour of its run's phase, and
a commit another room made is quieter. The picked row is washed and its
identity opens under it, each value whole on one line and cut by the
pane's edge, so a copy takes the whole value. Every role and `aria-*`
value comes in the bags. -->
<script lang="ts">
  import { PHASE_FILL } from "../runs/phase";
  import type { CommitsLook } from "./commits";

  const look: CommitsLook = $props();
</script>

<ol>
  {#each look.rows as row (row.key)}
    <li>
      <button
        {...row.wire}
        class={[
          "-mr-snug grid h-bar w-full grid-cols-[auto_8ch_minmax(0,1fr)] items-center gap-x-base rounded-card pr-snug text-left text-note",
          row.picked ? "wash-strong" : "hover:wash",
          row.here ? "text-text" : "text-text-faint",
        ]}
      >
        <span class="relative h-bar" style:width="{look.width}px" aria-hidden="true">
          <svg class="absolute inset-0 overflow-visible" width={look.width} height={look.height} viewBox="0 0 {look.width} {look.height}">
            {#each row.lines as line, at (at)}
              <path d={line.d} fill="none" stroke-width="2" class={line.mine ? "stroke-accent" : "stroke-edge-input"} />
            {/each}
          </svg>
          <span
            class={[
              "absolute top-1/2 size-dot -translate-x-1/2 -translate-y-1/2 rounded-pill",
              PHASE_FILL[row.node.phase],
              row.node.mine ? "ring-2 ring-accent" : "",
            ]}
            style:left="{row.node.left}px"
          ></span>
        </span>
        <span class="figure text-text-quiet">{row.short}</span>
        <span class="truncate">
          {row.message}
          <span class="ml-snug text-text-faint">{row.room}</span>
        </span>
      </button>
      {#if row.facts !== undefined}
        <div class="mb-base ml-[calc(var(--spacing-base)+8ch)] border-l border-edge-panel pl-base text-note">
          <dl class="grid grid-cols-[8ch_minmax(0,1fr)] gap-x-base text-text-quiet">
            <dt class="text-text-faint">{look.labels.oid}</dt>
            <dd class="figure truncate">{row.facts.oid}</dd>
            {#if row.facts.b3 !== undefined}
              <dt class="text-text-faint">{look.labels.b3}</dt>
              <dd class="figure truncate">{row.facts.b3}</dd>
            {/if}
            <dt class="text-text-faint">{look.labels.parents}</dt>
            <dd class="figure truncate">{row.facts.parents}</dd>
          </dl>
        </div>
      {/if}
    </li>
  {/each}
</ol>
{#if look.more !== undefined}
  <a class="block py-snug text-note text-accent hover:text-accent-hover" href={look.more.href}>{look.more.word}</a>
{/if}
