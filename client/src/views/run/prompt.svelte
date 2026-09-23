<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // What this run was told, in the four segments it was sent as. Each
  // one folds open, states the files it was read from and what the
  // budget cut off the end of them, and copies as the text the model
  // saw.
  //
  // The bytes come from the store, addressed by the hash the ledger
  // recorded; a segment the store no longer holds says so rather than
  // showing an empty box a reader would take for an empty prompt.

  import type { Key } from "../../core/lang";
  import type { PrefixSlot } from "../../wire";

  const SLOT_WORD: Record<PrefixSlot, Key> = {
    city: "slot_city",
    building: "slot_building",
    resident: "slot_resident",
    run: "slot_run",
  };
</script>

<script lang="ts">
  import { SvelteSet } from "svelte/reactivity";

  import { buildingOf } from "../../core/route";
  import { fill, say } from "../../core/lang";
  import { count } from "../../core/time";
  import { ui } from "../../ui";
  import type { PrefixSegment, RunId } from "../../wire";
  import Glyph from "../parts/glyph.svelte";
  import Path from "../parts/path.svelte";

  interface Props {
    readonly run: RunId;
  }

  const { run }: Props = $props();

  const u = ui();
  const lang = u.lang;

  // Each segment folds on its own: a person compares two segments and
  // should not lose the first while opening the second.
  const open = new SvelteSet<string>();
  // Which segment's copy receipt is showing right now (ux A7).
  let receipt = $state<string | null>(null);

  const asked = $derived(u.conn.asking.ask({ prefix: { run } }));
  const segments = $derived.by((): readonly PrefixSegment[] | undefined => {
    const held = $asked;
    if (held === undefined) return undefined;
    return "prefix" in held ? held.prefix.segments : [];
  });

  // A copy receipt lasts long enough to be seen and no longer (ux A7).
  $effect(() => {
    const at = receipt;
    if (at === null) return;
    const timer = setTimeout(() => {
      if (receipt === at) receipt = null;
    }, 1200);
    return () => {
      clearTimeout(timer);
    };
  });

  function copy(segment: PrefixSegment): void {
    void navigator.clipboard.writeText(segment.text);
    receipt = segment.hash;
  }

  function toggle(hash: string): void {
    if (!open.delete(hash)) open.add(hash);
  }
</script>

{#snippet segment(each: PrefixSegment)}
  {const isOpen = open.has(each.hash)}
  <li class="border-b border-edge">
    <div class="flex items-center gap-base py-snug text-note">
      <button
        type="button"
        class="flex min-w-0 flex-1 items-center gap-base text-left hover:text-text"
        aria-expanded={isOpen}
        onclick={() => {
          toggle(each.hash);
        }}
      >
        <Glyph
          name="chevron"
          size="sm"
          class="shrink-0 text-text-disabled transition-transform {isOpen ? 'rotate-90' : ''}"
        />
        <span class="shrink-0 text-text-quiet">{say($lang, SLOT_WORD[each.slot])}</span>
        <span class="shrink-0 text-text-disabled">
          {fill(say($lang, "run_prompt_bytes"), { n: count(each.bytes) })}
        </span>
        {#if !each.stored}
          <span class="truncate text-alert">{say($lang, "run_prompt_gone")}</span>
        {/if}
      </button>
      {#if each.stored}
        <!-- The receipt replaces the word for its moment (ux A7): the
             check is what a hand reads after the press, and the word
             is what it read before. -->
        <button
          type="button"
          class="inline-flex h-control-sm shrink-0 items-center gap-tight rounded-control bg-raised px-snug text-label text-text-quiet hover:bg-raised-hover hover:text-text"
          onclick={() => {
            copy(each);
          }}
        >
          {#if receipt === each.hash}
            <Glyph name="check" size="sm" />
            {say($lang, "run_prompt_copied")}
          {:else}
            {say($lang, "run_prompt_copy")}
          {/if}
        </button>
      {/if}
    </div>
    {#if isOpen}
      <div class="pb-base pl-wide">
        {#if each.sources.length > 0}
          <p class="flex flex-wrap items-baseline gap-snug pb-snug text-note text-text-faint">
            <span>{say($lang, "run_prompt_sources")}</span>
            {#each each.sources as source (source.addr)}
              <Path
                path={source.addr}
                onOpen={() => {
                  u.go({ kind: "building", address: buildingOf(source.addr) });
                }}
              />
              {#if source.dropped > 0}
                <span class="text-alert">
                  {fill(say($lang, "run_prompt_dropped"), { n: count(source.dropped) })}
                </span>
              {/if}
            {/each}
          </p>
        {/if}
        <pre
          class="overflow-x-auto whitespace-pre-wrap break-words rounded-control border border-edge bg-page p-base font-mono text-note text-text-quiet"
        >{each.text}</pre>
      </div>
    {/if}
  </li>
{/snippet}

{#if segments === undefined}
  <p class="text-text-disabled">…</p>
{:else if segments.length > 0}
  <ul>
    {#each segments as each (each.hash)}
      <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
      {@render segment(each)}
    {/each}
  </ul>
{:else}
  <p class="text-text-faint">{say($lang, "run_no_prompt")}</p>
{/if}
