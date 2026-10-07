<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // How the picked building's panel is drawn, and nothing else
  // (`./panel.ts`, `PanelLook`): its name and the close control, the
  // badges, the way into the building, the standing goal, the problems
  // and stuck lines, and the newest runs. The way in is a link in the
  // raised button's paint; it and the run lines change colour under the
  // pointer on the arrive curve and back on leave
  // (docs/frontend-method.md §4-43).
  import Badge from "../parts/badge.svelte";
  import Button from "../parts/button.svelte";
  import type { PanelLook } from "./panel";

  const look: PanelLook = $props();
</script>

<div class="flex flex-col gap-base">
  <div class="flex items-baseline gap-snug">
    <h2 class="min-w-0 flex-1 truncate text-heading font-heading">{look.name}</h2>
    <Button label={look.close.label} tone="quiet" onPress={look.close.onPress} />
  </div>
  {#if look.badges.length > 0}
    <div class="flex flex-wrap items-center gap-snug">
      {#each look.badges as badge (badge.key)}
        <Badge text={badge.text} weight={badge.weight} />
      {/each}
    </div>
  {/if}
  <a
    {...look.enter.wire}
    class="inline-flex h-control items-center self-start rounded-control bg-raised px-base text-label transition-colors ease-leave hover:bg-raised-hover hover:ease-arrive"
  >
    {look.enter.label}
  </a>
  {#if look.pursuit !== null}
    <p class="flex items-baseline gap-snug text-note">
      <span class={["inline-block size-dot shrink-0 rounded-pill", look.pursuit.running ? "bg-accent" : "bg-mark"]}></span>
      <span class="min-w-0 flex-1 text-text-quiet">{look.pursuit.goal}</span>
    </p>
  {/if}
  {#if look.problems.length > 0}
    <ul class="rounded-card border border-alert/40 px-base py-snug text-note text-text-quiet">
      {#each look.problems as problem (problem)}
        <li>{problem}</li>
      {/each}
    </ul>
  {/if}
  {#if look.blocked.length > 0}
    <ul class="text-note text-text-quiet">
      {#each look.blocked as line (line.key)}
        <li class="my-tight">
          <span class="text-alert">{line.source}</span> <span class="text-text-faint">{line.line}</span>
        </li>
      {/each}
    </ul>
  {/if}
  <section>
    <h3 class="mb-tight text-note text-text-faint">{look.runs.heading}</h3>
    {#if look.runs.rows.length > 0}
      <ul class="text-note">
        {#each look.runs.rows as run (run.key)}
          <li class="border-b border-edge py-snug">
            <a {...run.wire} class="group flex items-center gap-snug">
              <span class={["inline-block size-dot shrink-0 rounded-pill", run.moving ? "bg-accent" : "bg-mark"]}></span>
              <span class="min-w-0 flex-1 truncate text-text-quiet transition-colors ease-leave group-hover:text-text group-hover:ease-arrive">{run.title}</span>
              {#if run.at !== null}
                <span class="shrink-0 text-text-faint">{run.at}</span>
              {/if}
            </a>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="text-note text-text-faint">{look.runs.none}</p>
    {/if}
  </section>
</div>
