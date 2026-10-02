<script lang="ts" generics="T">
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Many rows of the same shape, chosen among and corrected in place: the
// table a provider's two hundred models are admitted from. A column
// carries its own three abilities - how a cell is drawn, whether the
// column can be ordered by, and whether its value can be corrected in
// place - and the empty state is the caller's snippet, because what to
// do about an empty table is knowledge of the page, not of the table.
//
// **A column states what it is worth; the table scrolls rather than
// squeezing it.** The header is one line whatever the column's width,
// every row is a control tall, and rows are separated by a hairline
// rather than by stripes: a band of alternating colour is one more
// thing competing with the values in the cells.

import type { Snippet } from "svelte";

import type { Column, Min, TableProps } from "./table";

const { caption, columns, rows, keyOf, selection, empty }: TableProps<T> =
  $props();

type Order = "up" | "down";

// The key of the column the rows are ordered by, and which way. The key
// rather than the column, so a column that leaves the table leaves its
// order behind instead of the state holding a column nobody drew.
let by = $state<string | null>(null);
let order = $state<Order>("up");

const ordered: readonly T[] = $derived.by(() => {
  if (by === null) return rows;
  const column = columns.find((each) => each.key === by);
  const compare = column === undefined ? undefined : column.compare;
  if (compare === undefined) return rows;
  const sorted = [...rows].sort(compare);
  return order === "up" ? sorted : sorted.reverse();
});

function turn(key: string): void {
  if (by === key) {
    order = order === "up" ? "down" : "up";
    return;
  }
  by = key;
  order = "up";
}

// What a sortable column's header states to a screen reader. A column
// that cannot be ordered by states nothing at all: `aria-sort` on it
// would be a sorting statement about a column nobody can sort.
function said(key: string): "ascending" | "descending" | "none" {
  if (by !== key) return "none";
  return order === "up" ? "ascending" : "descending";
}

// The class a column's stated minimum resolves to, and nothing at all
// when the column states none: the width rules of client-SPEC 4-33 have
// one spelling and this is it.
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

// Some rows picked and some not is neither "all" nor "none", and a box
// that says "none" while rows are ticked is a lie. `indeterminate` is a
// DOM property with no attribute spelling, so the state is derived here
// and handed to the element below.
const mixed = $derived.by((): boolean => {
  const chosen = selection;
  if (chosen === undefined) return false;
  return (
    !chosen.allPicked() && ordered.some((row) => chosen.picked(row))
  );
});

let headerBox = $state<HTMLInputElement | undefined>(undefined);

$effect(() => {
  if (headerBox !== undefined) {
    headerBox.indeterminate = mixed;
  }
});

// The checker types a `{#snippet}` name as a void call, which the
// lint lane rejects inside a render tag. The name is taken again as
// its `Snippet` type, and the template renders that.
const held: Snippet<Parameters<typeof drawHeld>> = drawHeld;
</script>

<!-- What one cell holds, apart from the box it is held in: either the
     column's own drawing, or the input a corrected column hands back. -->
{#snippet drawHeld(column: Column<T>, row: T)}
  {#if column.editable}
    {const editable = column.editable}
    <input
      class="w-full min-w-0 rounded-control bg-chrome px-snug py-tight font-mono text-note text-text"
      aria-label="{column.header} {keyOf(row)}"
      value={editable.text(row)}
      onchange={(event) => {
        editable.onEdit(row, event.currentTarget.value);
      }}
    />
  {:else}
    {@render column.render(row)}
  {/if}
{/snippet}

<div class="max-h-output w-full overflow-auto rounded-card border border-edge-panel">
  {#if rows.length > 0}
    <table class="w-full border-collapse text-body">
      <caption class="sr-only">{caption}</caption>
      <!-- No stacking number: a sticky header is positioned and the
           rows that scroll under it are not, so the header is painted
           last of the two. -->
      <thead class="sticky top-0 bg-raised">
        <tr class="border-b border-edge">
          {#if selection}
            {const chosen = selection}
            <th class="w-glyph px-base py-tight text-left">
              <input
                type="checkbox"
                bind:this={headerBox}
                aria-label={chosen.allLabel}
                checked={chosen.allPicked()}
                onchange={(event) => {
                  chosen.onPickAll(event.currentTarget.checked);
                }}
              />
            </th>
          {/if}
          {#each columns as column (column.key)}
            <th
              class={[
                "whitespace-nowrap px-base py-tight text-left text-note font-label text-text-quiet",
                breadthOf(column.min),
                column.summary === true ? "summary" : undefined,
              ]}
              aria-sort={column.compare === undefined
                ? undefined
                : said(column.key)}
            >
              {#if column.compare === undefined}
                {column.header}
              {:else}
                <button
                  type="button"
                  class="text-note font-label text-text-quiet hover:text-text"
                  onclick={() => {
                    turn(column.key);
                  }}
                >
                  {column.header}
                </button>
              {/if}
            </th>
          {/each}
        </tr>
      </thead>
      <tbody>
        {#each ordered as row (keyOf(row))}
          <tr
            class="h-control-lg border-b border-edge transition-[background-color] motion-reduce:transition-none hover:bg-raised-hover focus-within:bg-raised-hover last:border-b-0"
          >
            {#if selection}
              {const chosen = selection}
              <td class="px-base py-tight">
                <input
                  type="checkbox"
                  aria-label={keyOf(row)}
                  checked={chosen.picked(row)}
                  onchange={(event) => {
                    chosen.onPick(row, event.currentTarget.checked);
                  }}
                />
              </td>
            {/if}
            {#each columns as column (column.key)}
              <td
                class={[
                  "px-base py-tight text-text",
                  breadthOf(column.min),
                  column.summary === true ? "summary" : undefined,
                ]}
              >
                <!-- A figure column hands its control a box of the
                     figure's width. `width` on the cell states what the
                     column prefers; only a definite width on the box
                     **inside** the cell caps what the control
                     contributes to the column's minimum - an input asked
                     for its intrinsic width answers with its whole
                     default width, and a cell holding it grows past the
                     figure however small the column was declared to be. -->
                {#if column.min === "figure"}
                  <div class="w-figure">{@render held(column, row)}</div>
                {:else}
                  {@render held(column, row)}
                {/if}
              </td>
            {/each}
          </tr>
        {/each}
      </tbody>
    </table>
  {:else}
    {@render empty?.()}
  {/if}
</div>
