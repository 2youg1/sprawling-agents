<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The city in results-only mode: three figures - what waits for the
  // person, what failed, what just finished - then each group's newest
  // runs as rows that open the run. Only `FIRST` rows of a group are
  // drawn and the rest are a count, so the page costs the same for a
  // city of ten runs as for a city of thousands (core/results.ts).
  import { fill, say } from "../../core/lang";
  import { FIRST, resultsOf } from "../../core/results";
  import type { Outcome } from "../../core/results";
  import { toFragment } from "../../core/route";
  import { ago } from "../../core/time";
  import { ui } from "../../ui";

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;

  const groups = $derived(resultsOf(Object.values($belief.runs), FIRST));

  const INK: Record<Outcome, string> = {
    waiting: "text-alert",
    failed: "text-alert",
    done: "text-text-quiet",
  };
</script>

<div class="mx-auto w-full max-w-page">
  <div class="grid grid-cols-3 gap-base">
    {#each groups as group (group.outcome)}
      <div class="rounded-card border border-edge px-base py-snug">
        <div class="text-note {INK[group.outcome]}">{say($lang, `results_${group.outcome}`)}</div>
        <div class="text-figure text-text">{group.total}</div>
      </div>
    {/each}
  </div>
  {#each groups as group (group.outcome)}
    <section class="mt-wide" aria-label={say($lang, `results_${group.outcome}`)}>
      <h2 class="mb-snug text-label text-text">
        {say($lang, `results_${group.outcome}`)}
        <span class="text-text-faint">{group.total}</span>
      </h2>
      {#if group.total === 0}
        <p class="text-note text-text-disabled">{say($lang, "results_none")}</p>
      {:else}
        <ul>
          {#each group.first as run (run.run)}
            <li class="border-t border-edge">
              <a
                href={toFragment({ kind: "run", run: run.run })}
                class="flex items-baseline gap-base py-snug text-note hover:bg-chrome"
              >
                <span class="min-w-0 flex-1 truncate text-body text-text">{run.task ?? run.run}</span>
                {#if run.addr !== null}
                  <span class="shrink-0 truncate font-mono text-text-faint">{run.addr}</span>
                {/if}
                {#if run.started !== null}
                  <span class="shrink-0 text-text-faint">{ago($lang, run.started, u.now())}</span>
                {/if}
              </a>
            </li>
          {/each}
        </ul>
        {#if group.total > group.first.length}
          <p class="mt-snug text-note text-text-faint">
            {fill(say($lang, "results_more"), { n: String(group.total - group.first.length) })}
          </p>
        {/if}
      {/if}
    </section>
  {/each}
</div>
