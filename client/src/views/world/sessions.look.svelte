<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- How the sessions pane is drawn under its label: a row of tag toggles
when rows carry tags, then the groups, each a quiet heading over its
rows. A row is a state dot, its title with the pin when it is pinned and
its time, and under them the lines the session has to say; the row in
main is washed and carries the chosen bar on its leading edge, and the
row's menu key shows on hover, on focus and on the row in main. Every
link, role and `aria-*` value comes in the bags. -->
<script lang="ts">
  import Glyph from "../parts/glyph.svelte";
  import Tip from "../parts/tip.svelte";
  import ContextBar from "./context_bar.svelte";
  import SessionMenu from "./session_menu.svelte";
  import type { SessionsLook } from "./sessions";

  const look: SessionsLook = $props();
</script>

{#if look.filter !== undefined}
  <div {...look.filter.wire} class="mb-snug flex flex-wrap gap-tight">
    {#each look.filter.toggles as toggle (toggle.key)}
      <button
        {...toggle.wire}
        class="h-control-sm rounded-pill px-snug text-note text-text-quiet hover:wash hover:text-text aria-pressed:wash-strong aria-pressed:text-text"
      >
        {toggle.word}
      </button>
    {/each}
  </div>
{/if}
<!-- A row reaches out by one step on each side, so its wash and its
chosen bar stand outside the text; the column that scrolls is widened
by that step into the pane's own inset, so the pane clips neither. -->
<div class="-mx-snug min-h-0 flex-1 overflow-y-auto px-snug">
  {#each look.groups as group (group.key)}
    <h3 class="mt-base mb-tight text-note text-text-faint first:mt-0">{group.heading}</h3>
    <ul>
      {#each group.rows as row (row.key)}
        <li class={["row group relative -mx-snug flex items-start rounded-card", row.chosen ? "here wash-strong" : "hover:wash"]}>
          <a {...row.link} class="cells grid min-w-0 flex-1 gap-x-base rounded-card px-snug py-snug">
            <span class="dot mt-snug size-dot rounded-pill" data-dot={row.dot} aria-hidden="true"></span>
            <span class="flex min-w-0 items-center gap-tight">
              <span class={["truncate font-label", row.current ? "text-text" : "text-text-quiet"]} title={row.room}>{row.title}</span>
              {#if row.pin !== undefined}
                <Tip text={row.pin.hint}>
                  <span {...row.pin.wire} class="flex shrink-0 text-text-faint">
                    <Glyph name="pin" size="sm" />
                  </span>
                </Tip>
              {/if}
            </span>
            {#if !look.narrow}
              <span class="figure text-note text-text-faint">{row.when}</span>
              {#if row.phase !== undefined}
                <span class="col-start-2 col-end-4 truncate text-note text-text-quiet">{row.phase}</span>
              {/if}
              {#if row.delegated !== undefined}
                <span class="col-start-2 col-end-4 truncate text-note text-text-faint">{row.delegated}</span>
              {/if}
              <span class="col-start-2 col-end-4 line-clamp-2 text-note text-text-quiet">{row.about}</span>
              {#if row.ranOn !== undefined}
                <span class="col-start-2 col-end-4 truncate text-note text-text-faint">{row.ranOn}</span>
              {/if}
              {#if row.tags.length > 0}
                <span class="col-start-2 col-end-4 mt-tight flex flex-wrap gap-tight">
                  {#each row.tags as tag (tag)}
                    <span class="rounded-pill border border-edge-panel px-tight text-note text-text-faint">{tag}</span>
                  {/each}
                </span>
              {/if}
              {#if row.context !== undefined}
                <ContextBar room={row.context.room} run={row.context.run} />
              {/if}
            {/if}
          </a>
          <div
            class={[
              "shrink-0 pt-tight pr-tight",
              row.chosen ? "" : "opacity-0 transition-opacity ease-leave group-hover:opacity-100 group-hover:ease-arrive group-focus-within:opacity-100 group-focus-within:ease-arrive",
            ]}
          >
            <SessionMenu {...row.menu} />
          </div>
        </li>
      {/each}
    </ul>
  {/each}
</div>

<style>
  /* The dot's lane, the title's and the time's. The lane is a step
   * wider than the dot, so a halo round it is not cut by the title. */
  .cells {
    grid-template-columns: var(--spacing-base) minmax(0, 1fr) auto;
  }

  /* The state dot. A running session glows in the accent and one
   * waiting for the person in the alert, each a halo three eighths of
   * the dot wide; a done one is a ring, so the three read apart without
   * their colours. */
  .dot[data-dot="run"] {
    background-color: var(--color-accent);
    box-shadow: 0 0 0 calc(var(--spacing-dot) * 3 / 8) color-mix(in oklch, var(--color-accent) 14%, transparent);
  }
  .dot[data-dot="ask"] {
    background-color: var(--color-alert);
    box-shadow: 0 0 0 calc(var(--spacing-dot) * 3 / 8) color-mix(in oklch, var(--color-alert) 12%, transparent);
  }
  .dot[data-dot="done"] {
    border: calc(var(--spacing-dot) * 3 / 16) solid var(--color-mark);
  }

  /* The row in main carries the chosen bar down its leading edge,
   * standing in by one step from the row's top and bottom
   * (docs/frontend-method.md §7B). A border rather than a fill, so a
   * forced-colour mode keeps it. */
  .row.here::before {
    content: "";
    position: absolute;
    inset-block: var(--spacing-snug);
    inset-inline-start: 0;
    border-inline-start: var(--spacing-hair) solid var(--color-accent);
    border-radius: var(--radius-pill);
  }
</style>
