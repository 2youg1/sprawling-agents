<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  import Combobox from "../parts/combobox.svelte";
  import Field from "../parts/field.svelte";
  import { Table, type Column } from "../parts/table";
  import type {
    ChoiceLook,
    ColumnKey,
    FigureLook,
    ModelRowLook,
    ModelTableLook,
  } from "./model_table";

  const look: ModelTableLook = $props();

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
  <div class="flex flex-col gap-hair">
    <span class="block whitespace-nowrap font-mono text-note text-text">{row.id}</span>
    {#if row.named !== undefined}
      <span class="whitespace-nowrap text-note text-text-faint">{row.named}</span>
    {/if}
    {#if row.refusal !== undefined}
      <span role="alert" class="text-note text-alert">{row.refusal}</span>
    {/if}
  </div>
{/snippet}

{#snippet figureBox(box: FigureLook)}
  <div class="flex flex-col gap-hair">
    <Field
      label={box.label}
      labelling="hidden"
      kind="number"
      step={1024}
      mono
      value={box.value}
      placeholder={box.placeholder}
      onInput={box.input}
    />
    {#if box.value === "" && box.source !== ""}
      <span class="min-w-0 text-note text-text-faint">{box.source}</span>
    {/if}
  </div>
{/snippet}

{#snippet renderWindow(row: ModelRowLook)}
  <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
  {@render figureBox(row.window)}
{/snippet}

{#snippet renderCeiling(row: ModelRowLook)}
  <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
  {@render figureBox(row.ceiling)}
{/snippet}

{#snippet renderModalities(row: ModelRowLook)}
  <div class="flex flex-col gap-hair">
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
    {@render choose(row.input)}
    {#if row.said !== ""}
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
    <label
      class="flex h-control shrink-0 cursor-pointer items-center gap-tight whitespace-nowrap rounded-control px-snug text-text-quiet transition-[background-color] ease-leave hover:bg-raised hover:ease-arrive"
    >
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
  <Field
    label={look.manual.label}
    mono
    value={look.manual.value}
    placeholder={look.manual.placeholder}
    onInput={look.manual.input}
  />
</div>
