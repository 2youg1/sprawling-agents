<script lang="ts" generics="T">
  // This Source Code Form is subject to the terms of the Mozilla Public
  // License, v. 2.0. If a copy of the MPL was not distributed with this
  // file, You can obtain one at https://mozilla.org/MPL/2.0/.
  // Copyright (c) 2026 2youg1 and the sprawling contributors

  import type { Snippet } from "svelte";

  import type { HeldLook, Min, TableLook } from "./table";

  const look: TableLook<T> = $props();

  function breadthOf(min: Min | undefined): string | undefined {
    switch (min) {
      case undefined:
        return undefined;
      case "12ch":
        return "min-w-[12ch]";
      case "24ch":
        return "min-w-[24ch]";
      case "figure":
        return "w-figure";
    }
  }

  const held: Snippet<[HeldLook<T>]> = drawHeld;
</script>

{#snippet drawHeld(content: HeldLook<T>)}
  {#if content.kind === "field"}
    <input
      {...content.field}
      class="w-full min-w-0 rounded-control bg-chrome px-snug py-tight font-mono text-note text-text"
    />
  {:else if content.kind === "drawn"}
    {@render content.render(content.row)}
  {/if}
{/snippet}

<div class="max-h-output w-full overflow-auto rounded-card border border-edge-panel">
  {#if look.lines.length > 0}
    <table class="w-full border-collapse text-body">
      <caption class="sr-only">{look.caption}</caption>
      <thead class="sticky top-0 bg-raised">
        <tr class="border-b border-edge">
          {#if look.all !== undefined}
            <th class="w-glyph px-base py-tight text-left">
              <input {...look.all} />
            </th>
          {/if}
          {#each look.heads as head (head.key)}
            <th
              {...head.cell}
              class={[
                "whitespace-nowrap px-base py-tight text-left text-note font-label text-text-quiet",
                breadthOf(head.min),
                head.summary && "summary",
              ]}
            >
              {#if head.sort === undefined}
                {head.header}
              {:else}
                <button
                  {...head.sort}
                  class="text-note font-label text-text-quiet transition-colors ease-leave hover:text-text hover:ease-arrive"
                >
                  {head.header}
                </button>
              {/if}
            </th>
          {/each}
        </tr>
      </thead>
      <tbody>
        {#each look.lines as line (line.key)}
          <tr
            class="h-control-lg border-b border-edge transition-[background-color] ease-leave hover:wash hover:ease-arrive focus-within:wash focus-within:ease-arrive last:border-b-0"
          >
            {#if line.tick !== undefined}
              <td class="px-base py-tight">
                <input {...line.tick} />
              </td>
            {/if}
            {#each line.cells as cell (cell.key)}
              <td
                class={[
                  "px-base py-tight text-text",
                  breadthOf(cell.min),
                  cell.summary && "summary",
                ]}
              >
                {#if cell.min === "figure"}
                  <div class="figure-box">{@render held(cell.held)}</div>
                {:else}
                  {@render held(cell.held)}
                {/if}
              </td>
            {/each}
          </tr>
        {/each}
      </tbody>
    </table>
  {:else}
    {@render look.empty?.()}
  {/if}
</div>

<style>
  .figure-box {
    inline-size: calc(var(--spacing-figure) + var(--spacing-wide));
  }
</style>
