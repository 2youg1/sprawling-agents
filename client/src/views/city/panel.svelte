<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // What was picked on the drawing, in words: how far the plan has got,
  // what is stuck, the standing goal, and the runs that worked there.
  // It is the third column on a wide screen, a drawer over the drawing
  // on a medium one, and a block under the drawing on a narrow one -
  // one component either way, so the three widths cannot drift apart.

  import { heldWithin } from "../../core/belief/rooms";
  import { fill, say } from "../../core/lang";
  import { toFragment } from "../../core/route";
  import { clock } from "../../core/time";
  import { ui } from "../../ui";
  import type { Address, CityAnswer } from "../../wire";
  import Badge from "../parts/badge.svelte";
  import Button from "../parts/button.svelte";

  interface PanelProps {
    readonly addr: Address;
    readonly city: CityAnswer;
    readonly onClose: () => void;
  }

  const { addr, city, onClose }: PanelProps = $props();

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;

  const building = $derived(city.buildings.find((each) => each.addr === addr));
  // A building with no plan has no denominator: `null` rather than
  // zero, which would read as a plan nobody has started.
  const planned = $derived.by(() => {
    const held = building;
    if (held === undefined || !("planned" in held.progress)) return null;
    return held.progress.planned;
  });
  const pursuit = $derived(city.pursuits.find((line) => line.addr === addr));
  // The newest eight runs of the building, newest first.
  const runs = $derived(heldWithin($belief, addr).reverse().slice(0, 8));
</script>

<div class="flex flex-col gap-base">
  <div class="flex items-baseline gap-snug">
    <h2 class="min-w-0 flex-1 truncate text-heading font-heading">{addr}</h2>
    <Button label={say($lang, "panel_close")} tone="quiet" onPress={onClose} />
  </div>
  <div class="flex flex-wrap items-center gap-snug">
    {#if planned !== null}
      <Badge
        text={fill(say($lang, "bld_progress"), {
          done: String(planned.done),
          total: String(planned.total),
        })}
      />
      {#if building !== undefined && building.ready > 0}
        <Badge
          text={fill(say($lang, "city_ready"), { n: String(building.ready) })}
          weight="live"
        />
      {/if}
    {/if}
    {#if building !== undefined && building.blocked.length > 0}
      <Badge text={fill(say($lang, "city_stuck"), { n: String(building.blocked.length) })} weight="alert" />
    {/if}
  </div>
  <a
    href={toFragment({ kind: "building", address: addr })}
    class="inline-flex h-control items-center self-start rounded-control bg-raised px-base text-label hover:bg-raised-hover"
  >
    {say($lang, "city_enter")}
  </a>
  {#if pursuit !== undefined}
    <p class="flex items-baseline gap-snug text-note">
      <span
        class={[
          "inline-block size-dot shrink-0 rounded-pill",
          pursuit.state === "running" ? "bg-accent" : "bg-mark",
        ]}
      ></span>
      <span class="min-w-0 flex-1 text-text-quiet">{pursuit.goal}</span>
    </p>
  {/if}
  {#if building !== undefined && building.problems.length > 0}
    <ul class="rounded-card border border-alert/40 px-base py-snug text-note text-text-quiet">
      {#each building.problems as problem (problem)}
        <li>{problem}</li>
      {/each}
    </ul>
  {/if}
  {#if building !== undefined && building.blocked.length > 0}
    <ul class="text-note text-text-quiet">
      {#each building.blocked as line (line)}
        <li class="my-tight">
          <span class="text-alert">{line.source}</span> <span class="text-text-faint">{line.line}</span>
        </li>
      {/each}
    </ul>
  {/if}
  <section>
    <h3 class="mb-tight text-note text-text-faint">{say($lang, "city_runs")}</h3>
    {#if runs.length > 0}
      <ul class="text-note">
        {#each runs as run (run.run)}
          <li class="border-b border-edge py-snug">
            <a
              href={toFragment({ kind: "run", run: run.run })}
              class="flex items-center gap-snug hover:text-text"
            >
              <span
                class={[
                  "inline-block size-dot shrink-0 rounded-pill",
                  run.doing.kind === "frozen" ? "bg-mark" : "bg-accent",
                ]}
              ></span>
              <span class="min-w-0 flex-1 truncate text-text-quiet">{run.task ?? run.run}</span>
              {#if run.started !== null}
                <span class="shrink-0 text-text-faint">{clock($lang, run.started)}</span>
              {/if}
            </a>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="text-note text-text-faint">{say($lang, "tree_no_runs")}</p>
    {/if}
  </section>
</div>
