<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // A call as a terminal shows it (client/Spec.lean §7F): two lines of head -
  // what ran, then how long it took in milliseconds, how it ended, the
  // moment it finished and what the view cut - and under them what it
  // printed, stderr in the alert ink, never folded, scrolling sideways.
  //
  // **The original is one press away whenever the city kept one.** A call
  // whose output names a `pinned` locator offers it at the right of the
  // head, read through `Query::Content`, whether or not the view cut any
  // lines: a sieve can shorten an output without cutting a line, so `cut`
  // is no test for whether the original says more (roadmap §3-4). With no
  // locator the head says what was cut and that no original was kept,
  // rather than calling the view the whole output.
</script>

<script lang="ts">
  import { readable } from "svelte/store";
  import type { Readable } from "svelte/store";

  import { readAnswer } from "../../core/answered";
  import { count, kib } from "../../core/time";
  import { fill, say } from "../../core/lang";
  import type { Tail } from "../../core/live_output";
  import { ui } from "../../ui";
  import type { Answer, Call, Query } from "../../wire";
  import { INK, rows } from "../monitor/ansi";
  import type { Ending } from "../monitor/trace";
  import Unanswered from "../parts/unanswered.svelte";
  import { printedOf } from "./terminal";

  interface Props {
    readonly call: Call;
    // The running command's output so far, read only while the call waits.
    readonly tail: Tail;
  }

  const { call, tail }: Props = $props();

  const u = ui();
  const lang = u.lang;

  const printed = $derived(printedOf(call, tail));
  const drawn = $derived([
    ...(printed.out === "" ? [] : rows(printed.out).map((row) => ({ alert: false, row }))),
    ...(printed.err === "" ? [] : rows(printed.err).map((row) => ({ alert: true, row }))),
  ]);

  let raw = $state(false);
  const NOTHING: Readable<Answer | undefined> = readable(undefined);
  const rawQuestion = $derived.by((): Query | null => (raw && printed.pinned !== null ? { content: { locator: printed.pinned } } : null));
  const rawAsked = $derived(rawQuestion === null ? NOTHING : u.conn.asking.ask(rawQuestion));
  const original = $derived(readAnswer($rawAsked, (held) => ("content" in held ? held.content : undefined)));

  function ended(ending: Ending): { readonly word: string; readonly ink: string } {
    switch (ending.kind) {
      case "code":
        return { word: fill(say($lang, "mon_exited"), { code: String(ending.code) }), ink: ending.code === 0 ? "text-accent" : "text-alert" };
      case "stopped":
        return { word: fill(say($lang, "mon_stopped"), { why: ending.why }), ink: "text-alert" };
      case "running":
        return { word: say($lang, "mon_running"), ink: "text-accent" };
      case "unread":
        return { word: say($lang, "mon_unread"), ink: "" };
    }
  }
</script>

<div class="flex min-h-0 flex-1 flex-col bg-chrome">
  <div
    class="grid h-[calc(8*var(--spacing-baseline))] shrink-0 grid-cols-[minmax(0,1fr)_auto] grid-rows-[calc(3*var(--spacing-baseline))_calc(2*var(--spacing-baseline))] content-center gap-x-pane border-b border-edge pr-snug pl-wide"
  >
    <p class="truncate font-mono text-note text-text">
      <span class="mr-base font-label">{call.tool}</span>{printed.line}
    </p>
    {#if printed.pinned !== null}
      <button
        type="button"
        class="row-span-2 h-control-sm self-center rounded-control px-snug text-note text-text-quiet hover:bg-raised hover:text-text aria-pressed:bg-raised aria-pressed:text-text"
        aria-pressed={raw}
        onclick={() => {
          raw = !raw;
        }}>{say($lang, "inspect_raw")}</button
      >
    {/if}
    <p class="col-start-1 flex gap-pane overflow-hidden font-mono text-note leading-[calc(2*var(--spacing-baseline))] whitespace-nowrap text-text-faint">
      {#if printed.took !== null}<span class="figure">{fill(say($lang, "inspect_took"), { ms: count(printed.took) })}</span>{/if}
      {#if printed.ending !== null}
        {@const end = ended(printed.ending)}
        <span class={end.ink}>{end.word}</span>
      {/if}
      {#if printed.finished !== null}
        {@const iso = new Date(printed.finished).toISOString()}
        <time class="figure" datetime={iso}>{iso.slice(11)}</time>
      {/if}
      {#if printed.cut > 0}
        <span>{fill(say($lang, "inspect_cut"), { n: count(printed.cut) })}</span>
        {#if printed.pinned === null}<span>{say($lang, "inspect_no_original")}</span>{/if}
      {/if}
    </p>
  </div>
  <div class="min-h-0 flex-1 overflow-auto">
    {#if rawQuestion !== null}
      {#if original.kind === "held"}
        {#if original.value.binary}
          <p class="px-wide py-snug text-note text-text-faint">{fill(say($lang, "inspect_not_text"), { bytes: kib(original.value.bytes) })}</p>
        {:else}
          {#if original.value.truncated}
            <p class="px-wide pt-snug text-note text-text-faint">{fill(say($lang, "inspect_raw_head"), { bytes: kib(original.value.bytes) })}</p>
          {/if}
          <pre class="w-max min-w-full px-wide py-snug font-mono text-note leading-[calc(3*var(--spacing-baseline))] text-text-quiet">{original.value.text}</pre>
        {/if}
      {:else if original.kind === "unavailable"}
        <div class="px-wide py-snug"><Unanswered query={original.query} asked={rawQuestion} /></div>
      {:else}
        <p class="px-wide py-snug text-note text-text-faint">…</p>
      {/if}
    {:else}
      <div class="w-max min-w-full px-wide py-snug font-mono text-note leading-[calc(3*var(--spacing-baseline))] text-text-quiet" aria-live="off">
        {#each drawn as line, r (r)}
          <div class="min-h-[1lh] whitespace-pre">{#each line.row as piece, p (p)}<span
                class="{line.alert ? 'text-alert' : INK[piece.tone]} {piece.bold ? 'font-semibold' : ''}">{piece.text}</span
              >{/each}</div>
        {/each}
      </div>
    {/if}
  </div>
</div>
