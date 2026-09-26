<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The city in results-only mode: a filter over the outcomes with the
  // count of each, then the runs it keeps as one list read by time and
  // cut into recency bands. A row states when the run began, how it
  // ended, its room, its task, and what it produced or why it stopped.
  // Only `FIRST` runs of an outcome are drawn and the tab keeps the whole
  // count, so the page costs the same for a city of ten runs as for a
  // city of thousands (core/results.ts).
  import Glyph from "../parts/glyph.svelte";
  import type { GlyphName } from "../parts/glyph";
  import Segmented from "../parts/segmented.svelte";
  import type { Choice } from "../parts/segmented";
  import Produced from "./produced.svelte";
  import { fill, say } from "../../core/lang";
  import { FIRST, bandsOf, outcomeOf, resultsOf } from "../../core/results";
  import type { Outcome } from "../../core/results";
  import { toFragment } from "../../core/route";
  import { ui } from "../../ui";

  type Tab = "all" | Outcome;

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;

  let tab = $state<Tab>("all");

  const runs = $derived(Object.values($belief.runs));
  const groups = $derived(resultsOf(runs, FIRST));
  const listed = $derived(groups.reduce((sum, group) => sum + group.total, 0));
  const tabs = $derived<readonly Choice<Tab>[]>([
    { value: "all", label: fill(say($lang, "results_tab"), { name: say($lang, "results_all"), n: String(listed) }) },
    // "ended" is the posture a reload leaves every frozen run in; it
    // earns a tab only when some run is in it.
    ...groups
      .filter((group) => group.outcome !== "ended" || group.total > 0)
      .map((group) => ({
      value: group.outcome,
      label: fill(say($lang, "results_tab"), {
        name: say($lang, `results_${group.outcome}`),
        n: String(group.total),
      }),
    })),
  ]);
  const bands = $derived(
    bandsOf(
      groups.filter((group) => tab === "all" || group.outcome === tab).flatMap((group) => group.first),
      u.now(),
    ),
  );

  const GLYPH: Record<Outcome, GlyphName> = { waiting: "hand", failed: "cross", done: "check", ended: "ring" };
  const INK: Record<Outcome, string> = {
    waiting: "text-alert",
    failed: "text-alert",
    done: "text-text-quiet",
    ended: "text-text-faint",
  };

  function hhmm(at: number): string {
    return new Date(at).toLocaleTimeString("en-GB", { hour: "2-digit", minute: "2-digit" });
  }
</script>

<div class="mx-auto w-full max-w-page">
  <p class="text-note text-text-quiet">
    {fill(say($lang, "results_city"), { runs: String(runs.length), rest: String(runs.length - listed) })}
  </p>
  <div class="mt-snug">
    <Segmented
      label={say($lang, "results_which")}
      options={tabs}
      held={tab}
      onPick={(value) => {
        tab = value;
      }}
    />
  </div>
  {#if bands.length === 0}
    <p class="mt-wide text-note text-text-disabled">{say($lang, "results_none")}</p>
  {/if}
  {#each bands as band (band.recency)}
    <section class="mt-wide" aria-label={say($lang, `results_${band.recency}`)}>
      <h2 class="mb-tight border-b border-edge pb-tight text-note text-text-quiet">
        {say($lang, `results_${band.recency}`)}
        <span class="text-text-faint">{band.runs.length}</span>
      </h2>
      <ul>
        {#each band.runs as run (run.run)}
          {@const outcome = outcomeOf(run)}
          <li class="border-b border-l-2 border-b-edge {outcome === 'waiting' ? 'border-l-alert' : 'border-l-transparent'}">
            <a
              href={toFragment({ kind: "run", run: run.run })}
              class="flex items-baseline gap-base px-tight py-snug text-note hover:bg-chrome"
            >
              <span class="w-figure shrink-0 font-mono text-text-faint">{run.started === null ? "" : hhmm(run.started)}</span>
              {#if outcome !== null}
                <span class="shrink-0 self-center {INK[outcome]}"><Glyph name={GLYPH[outcome]} size="sm" /></span>
              {/if}
              <span class="w-output shrink-0 truncate font-mono text-text-quiet">{run.addr ?? ""}</span>
              <span class="min-w-0 flex-1 truncate text-body text-text">{run.task ?? run.run}</span>
              {#if outcome === "done"}
                <Produced run={run.run} />
              {:else if outcome === "failed" && run.doing.kind === "frozen" && run.doing.completion !== null}
                <span class="shrink-0 text-text-quiet">{run.doing.completion}</span>
              {/if}
            </a>
          </li>
        {/each}
      </ul>
    </section>
  {/each}
</div>
