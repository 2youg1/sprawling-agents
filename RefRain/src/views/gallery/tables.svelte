<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  import type { Snippet } from "svelte";

  import type { Lang } from "../../core/lang";
  import { STOP } from "../../core/slash";
  import { fill, say } from "../../core/lang";
  import type { Column } from "../parts/table";

  interface Served {
    readonly id: string;
    readonly context: string;
    readonly ceiling: string;
    readonly modalities: string;
    readonly price: string;
    readonly role: string;
  }

  const LONG = "acme-inference/orion-3-ultra-preview-2026-09";

  const SERVED: readonly Served[] = [
    {
      id: LONG,
      context: "1024000",
      ceiling: "65536",
      modalities: "image text",
      price: "0.000003 / 0.000015",
      role: "main",
    },
    {
      id: "openai/gpt-nucleus-6",
      context: "400000",
      ceiling: "—",
      modalities: "text",
      price: "—",
      role: "—",
    },
    {
      id: "meta/muse-spark-1.3-contributor",
      context: "131072",
      ceiling: "8192",
      modalities: "text",
      price: "0.000001 / 0.000004",
      role: "digest",
    },
  ];

  function keyOf(row: Served): string {
    return row.id;
  }

  interface Cells {
    readonly id: Snippet<[Served]>;
    readonly context: Snippet<[Served]>;
    readonly ceiling: Snippet<[Served]>;
    readonly modalities: Snippet<[Served]>;
    readonly price: Snippet<[Served]>;
    readonly role: Snippet<[Served]>;
  }

  function columnsOf(lang: Lang, cells: Cells): readonly Column<Served>[] {
    return [
      {
        key: "id",
        header: say(lang, "part_model_id"),
        min: "24ch",
        render: cells.id,
        compare: (a, b) => a.id.localeCompare(b.id),
      },
      {
        key: "context",
        header: say(lang, "part_context_window"),
        min: "figure",
        render: cells.context,
        compare: (a, b) => Number(a.context) - Number(b.context),
      },
      {
        key: "ceiling",
        header: say(lang, "part_output_ceiling"),
        min: "figure",
        render: cells.ceiling,
        compare: (a, b) => Number(a.ceiling) - Number(b.ceiling),
      },
      {
        key: "modalities",
        header: say(lang, "setup_model_modalities"),
        min: "12ch",
        render: cells.modalities,
      },
      {
        key: "price",
        header: say(lang, "setup_model_price"),
        min: "12ch",
        render: cells.price,
      },
      {
        key: "role",
        header: say(lang, "setup_model_role"),
        min: "figure",
        render: cells.role,
      },
    ];
  }
</script>

<script lang="ts">
  import { ui } from "../../ui";
  import Badge from "../parts/badge.svelte";
  import Button from "../parts/button.svelte";
  import EmptyState from "../parts/empty.svelte";
  import Path from "../parts/path.svelte";
  import Row, { RowList } from "../parts/row.svelte";
  import { Table } from "../parts/table";
  import Case from "./case.svelte";

  const { lang } = ui();
</script>

{#snippet idCell(row: Served)}
  <span class="block whitespace-nowrap font-mono text-note">{row.id}</span>
{/snippet}

{#snippet contextCell(row: Served)}
  <span class="font-mono text-note">{row.context}</span>
{/snippet}

{#snippet ceilingCell(row: Served)}
  <span class="font-mono text-note">{row.ceiling}</span>
{/snippet}

{#snippet modalitiesCell(row: Served)}
  <span class="text-note text-text-faint">{row.modalities}</span>
{/snippet}

{#snippet priceCell(row: Served)}
  <span class="block whitespace-nowrap font-mono text-note text-text-faint">{row.price}</span>
{/snippet}

{#snippet roleCell(row: Served)}
  <span class="font-mono text-note">{row.role}</span>
{/snippet}

<Case label="table · a 24ch id column inside a 320px container" width={320}>
  <div class="min-w-0">
    <Table
      caption={say($lang, "setup_models")}
      columns={columnsOf($lang, {
        id: idCell,
        context: contextCell,
        ceiling: ceilingCell,
        modalities: modalitiesCell,
        price: priceCell,
        role: roleCell,
      })}
      rows={SERVED}
      {keyOf}
    />
  </div>
</Case>

<Case label="table · six columns inside 520px, one row ticked" width={520}>
  <div class="min-w-0">
    <Table
      caption={say($lang, "setup_models")}
      columns={columnsOf($lang, {
        id: idCell,
        context: contextCell,
        ceiling: ceilingCell,
        modalities: modalitiesCell,
        price: priceCell,
        role: roleCell,
      })}
      rows={SERVED}
      {keyOf}
      selection={{
        picked: (row) => row.id === LONG,
        onPick: () => undefined,
        allLabel: say($lang, "setup_select_all"),
        allPicked: () => false,
        onPickAll: () => undefined,
      }}
    />
  </div>
</Case>

<Case label="path · an address too long for a 240px column" width={240}>
  <div class="min-w-0 rounded-card border border-edge-panel px-base py-snug">
    <Path path="city/lab/west/experiments/2026/07/completions-with-a-long-name.md" />
  </div>
</Case>

{#snippet ledgerRows()}
  <Row
    primary={LONG}
    secondary={fill(say($lang, "talk_tokens"), { n: "12480" })}
    onOpen={() => undefined}
  >
    {#snippet status()}
      <Badge text={say($lang, "status_in_progress")} weight="live" dot />
    {/snippet}
    {#snippet actions()}
      <Button label={STOP} tone="quiet" />
    {/snippet}
  </Row>
  <Row primary="openai/gpt-nucleus-6" secondary={say($lang, "tree_no_runs")}>
    {#snippet status()}
      <Badge text={say($lang, "city_quiet")} dot />
    {/snippet}
  </Row>
{/snippet}

<Case label="rows · a long name, a second line and a right-hand action">
  <div class="rounded-card border border-edge-panel">
    <!-- eslint-disable-next-line @typescript-eslint/no-unsafe-call (the walk is a snippet exported from a module script; svelte-check types this call, while eslint's own pass cannot see through the component file) -->
    {@render RowList({ label: say($lang, "rec_ledger"), rows: ledgerRows })}
  </div>
</Case>

<Case label="rows · nothing in the list">
  <div class="rounded-card border border-edge-panel">
    <EmptyState missing="rec_nothing">
      {#snippet action()}
        <Button label={say($lang, "city_ask_mayor")} tone="primary" />
      {/snippet}
    </EmptyState>
  </div>
</Case>
