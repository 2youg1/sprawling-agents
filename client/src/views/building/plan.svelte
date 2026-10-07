<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The plan as rows: what the building set out to do, how deep each
  // item sits, what it waits for, and where it stands. What the city
  // could not read is stated above the plan rather than swallowed, and
  // what is stuck is stated below it with the line that said so.
  //
  // A row is a disclosure rather than a table row: it opens into what
  // that node has cost and which runs spent it (`Query::CostOf`,
  // client/Spec.lean §4-50), which a sortable table has no place for.
  //
  // The state on the right is `parts/badge.svelte`. It was five
  // hand-drawn paints here, which is the same thing that component is
  // for, and the nested conditional that chose between them had no arm
  // for a row awaiting approval - that row was drawn as one nobody had
  // started.

  import type { Key } from "../../core/lang";
  import { fill, say } from "../../core/lang";
  import { percent } from "../../core/share";
  import { ui } from "../../ui";
  import type { BuildingAnswer, PlanRow, RoadmapStatus } from "../../wire";
  import Badge from "../parts/badge.svelte";
  import type { Weight } from "../parts/glyph";
  import Progress from "../parts/progress.svelte";
  import { disclosureWire } from "./disclosure";
  import Disclosure from "./disclosure.look.svelte";
  import NodeCost from "./node_cost.svelte";

  interface Props {
    readonly answer: BuildingAnswer;
  }

  const { answer }: Props = $props();

  const lang = ui().lang;

  // The row a hand opened: one node's cost question at a time.
  let open = $state<string | null>(null);

  const STATUS_WORD: Record<RoadmapStatus, Key> = {
    not_started: "status_not_started",
    in_progress: "status_in_progress",
    done: "status_done",
    blocked: "status_blocked",
    awaiting_approval: "status_awaiting_approval",
  };

  function statusWord(row: PlanRow): string {
    return say(
      $lang,
      row.status === "not_started" && row.ready ? "status_ready" : STATUS_WORD[row.status],
    );
  }

  // How loudly one row's state is drawn. Exhaustive over the five
  // states the wire carries, so a sixth is a compile error rather than
  // a row that quietly looks like an idle one. The colour only repeats
  // the word beside it, which is why two states may share a weight:
  // `ready` and `in_progress` are both the city working, and `blocked`
  // and `awaiting_approval` are both the city stopped until somebody
  // acts (client/Spec.lean §4-32).
  function weightOf(row: PlanRow): Weight {
    switch (row.status) {
      case "in_progress":
        return "live";
      case "blocked":
      case "awaiting_approval":
        return "alert";
      case "not_started":
        return row.ready ? "live" : "quiet";
      case "done":
        return "quiet";
    }
  }

  // One pane of indent per level of the node number, so the plan
  // tightens with the rest of the page instead of holding a width of
  // its own.
  function indentOf(node: string): string {
    return `calc(var(--spacing-pane) * ${String(node.split(".").length - 1)})`;
  }
</script>

<div>
  <h2 class="mb-base text-note text-text-faint">{say($lang, "bld_plan")}</h2>
  <!-- Two figures, because either alone misleads (kernel::completion).
       The bar is the leaves, which say how many pieces the plan turned
       out to have; the figure beside it is the weighted share, which
       moves when a branch is divided generously. A reader seeing both
       can tell work finished from work redistributed. -->
  <!-- A plan with no leaves has no progress to show: a bar at zero
       reads as work stalled rather than work not yet planned. -->
  {#if "planned" in answer.progress && answer.progress.planned.total > 0}
    <div class="mb-base flex items-center gap-base">
      <Progress
        label={say($lang, "plan_progress")}
        done={answer.progress.planned.done}
        total={answer.progress.planned.total}
      />
      <span class="shrink-0 font-mono text-note text-text-faint"
        >{fill(say($lang, "plan_share"), {
          percent: String(percent(answer.progress.planned.done_ppb)),
        })}</span
      >
    </div>
  {:else if "unplanned" in answer.progress}
    <p class="mb-base text-note text-text-faint">
      {fill(say($lang, "plan_unplanned"), { steps: String(answer.progress.unplanned.steps) })}
    </p>
  {/if}
  {#if answer.problems.length > 0}
    <ul class="mb-base rounded-card border border-alert/40 px-base py-snug text-note text-text-quiet">
      {#each answer.problems as problem (problem)}
        <li>{problem}</li>
      {/each}
    </ul>
  {/if}
  {#if answer.plan.length > 0}
    <ul class="border-t border-edge text-note">
      {#each answer.plan as row (row.node)}
        <li class="border-b border-edge">
          {#snippet cells()}
            <span class="flex min-w-0 items-baseline gap-base" style:padding-left={indentOf(row.node)}>
              <span class="w-figure shrink-0 figure text-text-faint">{row.node}</span>
              <span class={row.status === "done" ? "text-text-faint" : "text-text"}>
                {row.item}
                {#if row.needs.length > 0}
                  <span class="ml-snug text-text-faint">← {row.needs.join(", ")}</span>
                {/if}
              </span>
            </span>
            <Badge text={statusWord(row)} weight={weightOf(row)} dot />
          {/snippet}
          <Disclosure
            layout="plan"
            open={open === row.node}
            wire={disclosureWire(open === row.node, () => {
              open = open === row.node ? null : row.node;
            })}
            {cells}
          />
          {#if open === row.node}
            <div class="pb-base pl-[calc(var(--spacing-snug)+var(--spacing-glyph-sm)+var(--spacing-figure)+var(--spacing-base)*2)]">
              <NodeCost building={answer.addr} node={row.node} />
            </div>
          {/if}
        </li>
      {/each}
    </ul>
  {:else}
    <p class="text-text-faint">{say($lang, "plan_empty")}</p>
  {/if}
  {#if answer.blocked.length > 0}
    <ul class="mt-base text-note text-text-quiet">
      {#each answer.blocked as line (line.source)}
        <li class="my-tight">
          <span class="text-alert">{line.source}</span> · {line.line}
        </li>
      {/each}
    </ul>
  {/if}
</div>
