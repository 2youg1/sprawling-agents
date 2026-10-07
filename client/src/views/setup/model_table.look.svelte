<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // How the probed-model table is drawn, and nothing else: every word
  // arrives translated and every press arrives wired (`./model_table`,
  // `ModelTableLook`), so another look draws the same table by taking
  // the same value.
  //
  // The table itself is `parts/table.svelte`: the sticky header, the
  // ordering, the tick column and the scroll box are that part's. Every
  // control a row holds stands at the one control height, so the boxes
  // and the two choices in a row line up along one edge.
  import Combobox from "../parts/combobox.svelte";
  import Field from "../parts/field.svelte";
  import { Table, type Column } from "../parts/table";
  import type { ChoiceLook, ColumnKey, ModelRowLook, ModelTableLook } from "./model_table";

  const look: ModelTableLook = $props();

  // Every column states what it may not be drawn narrower than, and
  // nothing in a cell is broken across lines to make it fit. The id
  // column is the one that must never break: a model id is the whole of
  // what a person ticks, and the columns together are wider than the
  // measure this table sits in, so the container scrolls sideways
  // rather than squeezing the id into a vertical word
  // (docs/frontend-method.md §4-33).
  const columns = $derived(
    (look.table?.columns ?? []).map((column): Column<ModelRowLook> => ({
      key: column.key,
      header: column.header,
      min: column.min,
      render: cellOf(column.key),
      ...(column.compare === undefined ? {} : { compare: column.compare }),
    })),
  );

  function cellOf(key: ColumnKey) {
    switch (key) {
      case "id":
        return renderId;
      case "context":
        return renderWindow;
      case "output":
        return renderCeiling;
      case "modalities":
        return renderModalities;
      case "price":
        return renderPrice;
      case "role":
        return renderRole;
    }
  }
</script>

{#snippet choose(choice: ChoiceLook)}
  <Combobox
    label={choice.label}
    placeholder={choice.placeholder}
    empty={choice.empty}
    choices={choice.choices}
    value={choice.value}
    onPick={choice.pick}
  />
{/snippet}

{#snippet renderId(row: ModelRowLook)}
  <span class="block whitespace-nowrap font-mono text-note text-text">{row.id}</span>
{/snippet}

{#snippet renderWindow(row: ModelRowLook)}
  <Field
    label={row.window.label}
    labelling="hidden"
    kind="number"
    step={1024}
    mono
    value={row.window.value}
    placeholder={row.window.placeholder}
    onInput={row.window.input}
  />
{/snippet}

{#snippet renderCeiling(row: ModelRowLook)}
  <Field
    label={row.ceiling.label}
    labelling="hidden"
    kind="number"
    step={1024}
    mono
    value={row.ceiling.value}
    placeholder={row.ceiling.placeholder}
    onInput={row.ceiling.input}
  />
{/snippet}

<!-- The person's statement first, and under it the provider's own
     list, when it gave one, as the evidence. -->
{#snippet renderModalities(row: ModelRowLook)}
  <div class="flex flex-col gap-hair">
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
    {@render choose(row.input)}
    {#if row.said !== ""}
      <!-- wording-ok: the modalities the provider itself stated, spelled as the provider spelled them -->
      <span class="text-note text-text-faint">{row.said}</span>
    {/if}
  </div>
{/snippet}

{#snippet renderPrice(row: ModelRowLook)}
  <span class="block whitespace-nowrap font-mono text-note text-text-faint">{row.price}</span>
{/snippet}

{#snippet renderRole(row: ModelRowLook)}
  <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
  {@render choose(row.role)}
{/snippet}

{#snippet emptyRows()}
  <p class="px-base py-snug text-text-faint">{look.table?.empty}</p>
{/snippet}

<div class="flex flex-col gap-snug text-note">
  <!-- One row above the table, and it stays one row: the search box
       takes what the filter does not need, so a narrow column makes the
       box shorter rather than dropping the switch under it. -->
  <div class="flex items-center gap-snug">
    <div class="min-w-0 flex-1">
      <Field
        label={look.search.label}
        labelling="hidden"
        value={look.search.value}
        placeholder={look.search.placeholder}
        onInput={look.search.input}
      />
    </div>
    <label class="flex h-control shrink-0 cursor-pointer items-center gap-tight whitespace-nowrap rounded-control px-snug text-text-quiet transition-[background-color] ease-leave hover:bg-raised hover:ease-arrive">
      <input
        type="checkbox"
        checked={look.textOnly.on}
        onchange={(event) => {
          look.textOnly.toggle(event.currentTarget.checked);
        }}
      />
      {look.textOnly.label}
    </label>
  </div>
  <p class="text-text-faint">{look.count}</p>
  {#if look.table !== undefined}
    {@const table = look.table}
    <Table
      caption={table.caption}
      {columns}
      rows={table.rows}
      keyOf={(row: ModelRowLook) => row.id}
      empty={emptyRows}
      selection={table.tick}
    />
  {/if}
  {#if look.needed !== undefined}
    <p class="text-text-faint">{look.needed}</p>
  {/if}
  <Field
    label={look.manual.label}
    mono
    value={look.manual.value}
    placeholder={look.manual.placeholder}
    onInput={look.manual.input}
  />
</div>
