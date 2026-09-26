<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // One run as the results room draws it: when it began, how it ended,
  // what was asked, the last thing it said, and then what it left - the
  // files and lines it committed, or why it stopped and that it committed
  // nothing. The run's rounds hold the words and the two trees; the room
  // asks only for its own runs.
  import Glyph from "../parts/glyph.svelte";
  import Produced from "./produced.svelte";
  import { lastFenceIn } from "./trace";
  import { OUTCOME_GLYPH, OUTCOME_INK } from "../shared/outcome";
  import { fill, say } from "../../core/lang";
  import type { RunBelief } from "../../core/belief";
  import { outcomeOf } from "../../core/results";
  import { toFragment } from "../../core/route";
  import { hhmm } from "../../core/time";
  import { ui } from "../../ui";

  interface Props {
    readonly run: RunBelief;
  }

  const { run }: Props = $props();
  const u = ui();
  const { lang } = u;

  const outcome = $derived(outcomeOf(run));
  const asked = $derived(u.conn.asking.ask({ rounds: { run: run.run } }));
  const rounds = $derived.by(() => {
    const held = $asked;
    return held !== undefined && "rounds" in held ? held.rounds : null;
  });
  const quote = $derived([...(rounds?.turns ?? [])].reverse().find((turn) => (turn.said ?? "") !== "")?.said ?? null);
  const base = $derived(rounds?.opened_at ?? null);
  const head = $derived(rounds === null ? null : lastFenceIn(rounds.turns));
  const committed = $derived(base !== null && head !== null && head !== base);
  // Every part of the closing line after the first is set off by a dot,
  // drawn by the stylesheet so a screen reader reads the parts alone.
  const THEN = "before:mr-tight before:text-text-faint before:content-['·']";
  const why = $derived(run.doing.kind === "frozen" ? (run.doing.completion ?? "") : "");
</script>

<li class="mb-wide flex flex-col gap-tight">
  <div class="flex items-baseline gap-base">
    <span class="w-figure shrink-0 font-mono text-note text-text-faint">
      {run.started === null ? "" : hhmm(run.started)}
    </span>
    <span class="shrink-0 self-center {outcome === null ? 'text-accent' : OUTCOME_INK[outcome]}">
      <Glyph name={outcome === null ? "pulse" : OUTCOME_GLYPH[outcome]} size="sm" />
    </span>
    <span class="min-w-0 flex-1 text-body text-text">{run.task ?? run.run}</span>
  </div>
  <div class="ml-figure flex flex-col gap-tight border-l border-edge pl-base">
    {#if quote !== null}
      <p class="whitespace-pre-line pl-snug text-body text-text">{quote}</p>
    {/if}
    <p class="flex flex-wrap items-baseline gap-tight text-note text-text-quiet">
      {#if outcome === "done"}
        <span class="text-text">{say($lang, "results_block_done")}</span>
        {#if base !== null}
          <span class={THEN}><Produced {base} head={committed ? head : null} /></span>
        {/if}
        {#if run.pr !== null}
          <span class={THEN}>{fill(say($lang, "results_row_pr"), { pr: run.pr })}</span>
        {/if}
      {:else if outcome === "waiting"}
        <span class="text-alert">{say($lang, "results_waiting")}</span>
        {#if run.ask !== null}
          <span class="text-text {THEN}">{fill(say($lang, "results_row_ask"), { ask: run.ask })}</span>
        {/if}
      {:else if outcome === null && run.doing.kind !== "frozen"}
        <span class="text-accent">{say($lang, "results_block_working")}</span>
        <span class={THEN}>
          <a class="text-text underline" href={toFragment({ kind: "run", run: run.run })}>
            {say($lang, "results_block_open")}
          </a>
        </span>
      {:else}
        <span class="text-text">
          {fill(say($lang, outcome === "failed" ? "results_block_failed" : "results_block_stopped"), { why })}
        </span>
        {#if committed && base !== null}
          <span class={THEN}><Produced {base} {head} /></span>
        {:else}
          <span class={THEN}>{say($lang, "results_block_nothing")}</span>
        {/if}
      {/if}
    </p>
  </div>
</li>
