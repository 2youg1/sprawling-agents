<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The palette's list: places, or - once the line begins with `/` -
  // the verbs under the section each one names. The box owns the line,
  // the cursor and what a pick does; this draws the rows and reports
  // the pointer. A verb this place cannot run is greyed with its reason
  // where the hint goes, and stays focusable with `aria-disabled` so the
  // reason is reachable (7-2).
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import { Kbd } from "../parts/kbd.svelte";
  import type { Entry, Listing } from "./entry";
  import { SECTION_WORD } from "./sections";

  interface Props {
    readonly listing: Listing;
    // The flat list the cursor walks, in the order the rows are drawn.
    readonly shown: readonly Entry[];
    readonly cursor: number;
    readonly onHover: (at: number) => void;
    readonly onPick: (entry: Entry) => void;
  }

  const { listing, shown, cursor, onHover, onPick }: Props = $props();
  const { lang } = ui();
</script>

<ul class="mt-snug max-h-palette overflow-y-auto">
  {#if listing.kind === "verbs"}
    {#each listing.groups as group (group.section)}
      <li>
        <h2 class="px-base pt-snug text-label font-label text-text-faint">
          {say($lang, SECTION_WORD[group.section])}
        </h2>
      </li>
      {#each group.entries as entry (entry.label)}
        {@const at = shown.indexOf(entry)}
        <li>
          <button
            type="button"
            class={[
              "flex w-full items-center justify-between gap-snug rounded-control px-base py-snug text-left text-body",
              entry.why === undefined ? "hover:bg-raised" : "aria-disabled:text-text-disabled",
              at === cursor ? "bg-raised" : "",
            ]}
            aria-disabled={entry.why !== undefined}
            onmouseenter={() => {
              if (at >= 0) onHover(at);
            }}
            onclick={() => {
              onPick(entry);
            }}
          >
            <span class="truncate font-mono">{entry.label}</span>
            <span class="shrink-0 text-note text-text-faint">
              {entry.why === undefined ? entry.hint : say($lang, entry.why)}
            </span>
          </button>
        </li>
      {/each}
    {/each}
  {:else}
    {#each shown as entry, at (entry.label)}
      <li>
        <button
          type="button"
          class={[
            "flex w-full items-center justify-between gap-snug rounded-control px-base py-snug text-left text-body hover:bg-raised",
            at === cursor ? "bg-raised" : "",
          ]}
          onmouseenter={() => {
            onHover(at);
          }}
          onclick={() => {
            onPick(entry);
          }}
        >
          <span class="truncate font-mono">{entry.label}</span>
          <span class="shrink-0 text-note text-text-faint">
            {#if entry.action === undefined}
              {entry.hint}
            {:else}
              <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression, @typescript-eslint/no-unsafe-call (a snippet call is the render itself; svelte-check types this imported snippet fine, and typescript-eslint does not resolve exports of another .svelte module) -->
              {@render Kbd({ action: entry.action })}
            {/if}
          </span>
        </button>
      </li>
    {/each}
  {/if}
</ul>
