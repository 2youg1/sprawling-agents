<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The data table with a partial selection, sortable columns and a
  // cell corrected in place, and the same table with no rows.
  import { say } from "../../../core/lang";
  import { ui } from "../../../ui";
  import Button from "../../parts/button.svelte";
  import EmptyState from "../../parts/empty.svelte";
  import Table from "../../parts/table.svelte";
  import type { Column } from "../../parts/table";
  import Case from "../case.svelte";
  import { CHOSEN, MODELS } from "../served";
  import type { ModelRow } from "../served";

  const { lang } = ui();

  let rows = $state<readonly ModelRow[]>(MODELS);
  let picked = $state<readonly string[]>([CHOSEN.id]);

  const columns: readonly Column<ModelRow>[] = [
    {
      key: "id",
      header: say($lang, "part_model_id"),
      render: idCell,
      compare: (a, b) => a.id.localeCompare(b.id),
    },
    {
      key: "context",
      header: say($lang, "part_context_window"),
      render: contextCell,
      compare: (a, b) => Number(a.context) - Number(b.context),
    },
    {
      key: "ceiling",
      header: say($lang, "part_output_ceiling"),
      render: ceilingCell,
      editable: {
        text: (row) => row.ceiling,
        onEdit: (row, text) => {
          rows = rows.map((each) =>
            each.id === row.id ? { ...each, ceiling: text } : each,
          );
        },
      },
    },
  ];
</script>

{#snippet idCell(row: ModelRow)}
  <span class="font-mono text-note">{row.id}</span>
{/snippet}

{#snippet contextCell(row: ModelRow)}
  <span class="font-mono text-note">{row.context}</span>
{/snippet}

{#snippet ceilingCell(row: ModelRow)}
  <span class="font-mono text-note">{row.ceiling}</span>
{/snippet}

<Case label="table · partial selection, sortable, corrected in place">
  <Table
    caption={say($lang, "setup_models")}
    {columns}
    {rows}
    keyOf={(row: ModelRow) => row.id}
    selection={{
      picked: (row: ModelRow) => picked.includes(row.id),
      onPick: (row: ModelRow, on: boolean) => {
        picked = on ? [...picked, row.id] : picked.filter((each) => each !== row.id);
      },
      allLabel: say($lang, "part_select_all"),
      allPicked: () => picked.length === rows.length,
      onPickAll: (on: boolean) => {
        picked = on ? rows.map((each) => each.id) : [];
      },
    }}
  />
</Case>

<Case label="table · empty">
  <Table
    caption={say($lang, "setup_models")}
    {columns}
    rows={[]}
    keyOf={(row: ModelRow) => row.id}
  >
    {#snippet empty()}
      <EmptyState missing="setup_no_models">
        {#snippet action()}
          <Button label={say($lang, "setup_look")} tone="primary" />
        {/snippet}
      </EmptyState>
    {/snippet}
  </Table>
</Case>
