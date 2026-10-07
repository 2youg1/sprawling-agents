<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The context lens: how full each turn's window was, split into what
  // the cache held, what was read fresh and what was written, and the
  // files the run kept going back to, most consulted first. The window's
  // own list says what it holds; this says what the run leaned on.

  import { Option, Schema } from "effect";
  import { SvelteMap } from "svelte/reactivity";

  import { fill, say } from "../../core/lang";
  import { buildingOf } from "../../core/route";
  import { count } from "../../core/time";
  import { ui } from "../../ui";
  import { Address } from "../../wire";
  import type { Turn, Used } from "../../wire";
  import Path from "../parts/path.svelte";

  interface Props {
    readonly turns: readonly Turn[];
  }

  const { turns }: Props = $props();

  const u = ui();
  const lang = u.lang;

  const peak = $derived(peakOf(turns));
  const seen = $derived(seenOf(turns));

  function peakOf(round: readonly Turn[]): number {
    return Math.max(1, ...round.map((turn) => (turn.used?.input ?? 0) + (turn.used?.output ?? 0)));
  }

  function seenOf(round: readonly Turn[]): readonly (readonly [string, number])[] {
    const files = new SvelteMap<string, number>();
    for (const turn of round) {
      for (const call of turn.calls) {
        if ((call.tool === "read" || call.tool === "search") && call.subject !== null && call.subject !== undefined) {
          files.set(call.subject, (files.get(call.subject) ?? 0) + 1);
        }
      }
    }
    return [...files.entries()].sort((a, b) => b[1] - a[1]);
  }

  function barOf(used: Used | null | undefined, part: "cached" | "input" | "output"): number {
    if (used === null || used === undefined) return 0;
    const held = Math.min(used.cached ?? 0, used.input);
    const value = part === "cached" ? held : part === "input" ? used.input - held : used.output;
    return (value / peak) * 100;
  }

  // A file the run read is opened where the page can open it: the
  // building it belongs to. Which file the page then shows is not in
  // the address bar's vocabulary, so the path stops at the door.
  function opening(file: string): (() => void) | undefined {
    const at = Option.getOrNull(Schema.decodeOption(Address)(file));
    if (at === null) return undefined;
    return () => {
      u.go({ kind: "building", address: buildingOf(at) });
    };
  }
</script>

<div class="grid grid-cols-[repeat(auto-fit,minmax(320px,1fr))] gap-wide">
  <section>
    <h2 class="mb-base text-note text-text-faint">{say($lang, "run_window")}</h2>
    {#if turns.some((turn) => turn.used !== null && turn.used !== undefined)}
      <ul class="text-note">
        {#each turns as turn (turn.number)}
          <li class="my-tight flex items-center gap-snug">
            <span class="w-figure shrink-0 text-text-faint">{fill(say($lang, "run_turn_n"), { n: String(turn.number) })}</span>
            <!-- The legend under the list names the three colours, so a
                 hint on each segment would say a second time what the
                 page already says once. -->
            <span class="flex h-dot flex-1 overflow-hidden rounded-pill bg-track" aria-hidden="true">
              <span class="bg-mark" style:width="{barOf(turn.used, 'cached')}%"></span>
              <span class="bg-accent" style:width="{barOf(turn.used, 'input')}%"></span>
              <span class="bg-accent-solid" style:width="{barOf(turn.used, 'output')}%"></span>
            </span>
            <span class="figure w-figure shrink-0 text-right text-text-faint">
              {turn.used !== null && turn.used !== undefined ? count(turn.used.input + turn.used.output) : "—"}
            </span>
          </li>
        {/each}
      </ul>
      <p class="mt-snug flex flex-wrap gap-x-base text-note text-text-faint">
        <span class="inline-flex items-center gap-tight"
          ><span class="size-dot rounded-pill bg-mark"></span>{say($lang, "run_cached")}</span
        >
        <span class="inline-flex items-center gap-tight"
          ><span class="size-dot rounded-pill bg-accent"></span>{say($lang, "run_input")}</span
        >
        <span class="inline-flex items-center gap-tight"
          ><span class="size-dot rounded-pill bg-accent-solid"></span>{say($lang, "run_output")}</span
        >
      </p>
    {:else}
      <p class="text-text-faint">{say($lang, "run_no_usage")}</p>
    {/if}
  </section>
  <section>
    <h2 class="mb-base text-note text-text-faint">{say($lang, "run_read_files")}</h2>
    {#if seen.length > 0}
      <ul class="text-note">
        {#each seen as [file, n] (file)}
          <li class="my-tight flex items-center justify-between gap-base">
            <Path path={file} onOpen={opening(file)} />
            <span class="figure shrink-0 text-text-faint">{n > 1 ? `×${String(n)}` : ""}</span>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="text-text-faint">{say($lang, "run_read_nothing")}</p>
    {/if}
  </section>
</div>
