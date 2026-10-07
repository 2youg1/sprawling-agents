<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The list parts in each state they can be in: a walked list of rows
  // with its chosen one, the progress bar with and without an end, the
  // skeleton as prose and as a list, and the empty state in its three
  // seats. The tables are the table section's.

  import { fill, say } from "../../../core/lang";
  import { ui } from "../../../ui";
  import Badge from "../../parts/badge.svelte";
  import Button from "../../parts/button.svelte";
  import EmptyState from "../../parts/empty.svelte";
  import Progress from "../../parts/progress.svelte";
  import Row, { RowList } from "../../parts/row.svelte";
  import Skeleton from "../../parts/skeleton.svelte";
  import Case from "../case.svelte";

  const { lang } = ui();
</script>

{#snippet walkedRows()}
  <Row
    primary="openai/gpt-nucleus-6"
    secondary={fill(say($lang, "talk_tokens"), { n: "12480" })}
    onOpen={() => undefined}
  >
    {#snippet status()}
      <Badge text={say($lang, "status_in_progress")} weight="live" dot />
    {/snippet}
    {#snippet actions()}
      <Button label={say($lang, "mcp_check")} tone="quiet" />
    {/snippet}
  </Row>
  <Row primary="anthropic/claude-lattice" secondary={say($lang, "tree_no_runs")} onOpen={() => undefined} chosen />
  <Row primary="zenmux/local-mirror" secondary={say($lang, "tree_no_runs")}>
    {#snippet status()}
      <Badge text={say($lang, "city_quiet")} dot />
    {/snippet}
  </Row>
{/snippet}

<Case label="row · a walked list, the second row chosen">
  <div class="rounded-card border border-edge-panel">
    <!-- eslint-disable-next-line @typescript-eslint/no-unsafe-call (the walk is a snippet exported from a module script; svelte-check types this call, while eslint's own pass cannot see through the component file) -->
    {@render RowList({ label: say($lang, "mcp_servers"), rows: walkedRows })}
  </div>
</Case>

<Case label="progress · part of the way, at its end, and with no end yet">
  <div class="flex flex-col gap-base">
    <Progress label={say($lang, "machine_walk_progress")} done={3} total={12} />
    <Progress label={say($lang, "machine_walk_progress")} done={12} total={12} />
    <Progress label={say($lang, "machine_progress_label")} done={0} total={0} />
  </div>
</Case>

<Case label="progress · with no end yet, motion off">
  <div data-motion="off">
    <Progress label={say($lang, "machine_progress_label")} done={0} total={0} />
  </div>
</Case>

<Case label="skeleton · a paragraph, then a list of rows">
  <div class="flex flex-col gap-wide">
    <Skeleton label={say($lang, "machine_checking")} rows={4} />
    <Skeleton label={say($lang, "machine_checking")} rows={3} tall />
  </div>
</Case>

<Case label="empty · centred, with its way out">
  <EmptyState missing="rec_nothing">
    {#snippet action()}
      <Button label={say($lang, "city_ask_mayor")} tone="primary" />
    {/snippet}
  </EmptyState>
</Case>

<Case label="empty · in the region a list would take">
  <EmptyState missing="mcp_none" seat="region" />
</Case>

<Case label="empty · inset in a frame that draws the region">
  <div class="rounded-card border border-edge-panel">
    <EmptyState missing="rec_nothing" seat="inset" />
  </div>
</Case>
