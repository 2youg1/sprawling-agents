<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // How the capabilities section is drawn, and nothing else
  // (`./catalog.ts`, client/Spec.lean 4-50, its tenth part): which layer
  // a save writes, the bytes the newest run carried and what the draft
  // would change, one group per namespace with a control for the whole
  // group, and one row per tool with its own three-way control. A long
  // group starts folded, so a server of two hundred tools is one line
  // until a person opens it.
  import Button from "../parts/button.svelte";
  import Segmented from "../parts/segmented.svelte";
  import type { CatalogLook } from "./catalog";

  const look: CatalogLook = $props();
</script>

<div class="flex min-w-0 flex-col gap-wide">
  <div class="flex min-w-0 flex-col gap-snug">
    <span class="text-note text-text-quiet">{look.scopeLabel}</span>
    <Segmented label={look.scopeLabel} options={look.scopes} held={look.scope} onPick={look.onScope} />
    <p class="text-note">
      <span class="figure text-text-quiet">{look.meter}</span>
      {#if look.delta !== undefined}
        <span class="figure text-text">· {look.delta}</span>
      {/if}
    </p>
    <p class="text-note text-text-faint">{look.note}</p>
  </div>
  {#if look.empty !== undefined}
    <p class="text-note text-text-faint">{look.empty}</p>
  {:else}
    <ul class="flex min-w-0 flex-col gap-snug">
      {#each look.groups as group (group.key)}
        <li class="rounded-panel bg-raised p-snug">
          <details open={group.open} class="flex min-w-0 flex-col gap-snug">
            <summary class="flex min-w-0 cursor-pointer flex-wrap items-baseline gap-base text-note">
              <span class="font-mono text-text">{group.title}</span>
              <span class="min-w-0 flex-1 text-text-faint">{group.summary}</span>
            </summary>
            <div class="flex min-w-0 flex-col gap-snug pt-snug">
              <Segmented label={group.summary} options={group.choices} held={group.held} onPick={group.onPick} />
              <ul class="flex min-w-0 flex-col">
                {#each group.rows as row (row.key)}
                  <li class="flex min-w-0 flex-wrap items-center gap-x-base gap-y-tight border-t border-edge py-snug text-note">
                    <span class="flex min-w-0 grow basis-[16rem] flex-col gap-tight">
                      <span class="flex min-w-0 items-baseline gap-base">
                        <span class="shrink-0 font-mono text-text-quiet">{row.name}</span>
                        <span class="min-w-0 truncate text-text-faint">{row.hint}</span>
                      </span>
                      <span class="min-w-0 text-text-faint">{row.facts}</span>
                    </span>
                    <Segmented label={row.label} options={row.choices} held={row.held} onPick={row.onPick} />
                  </li>
                {/each}
              </ul>
            </div>
          </details>
        </li>
      {/each}
    </ul>
    <span class="flex flex-wrap items-center gap-base">
      <Button
        label={look.save.label}
        tone="primary"
        {...look.save.why === undefined ? {} : { why: look.save.why }}
        onPress={look.save.onPress}
      />
    </span>
  {/if}
</div>
