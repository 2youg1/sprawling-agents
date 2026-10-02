<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // What this city does without being asked, read only (refrain roadmap
  // UG, S06 Q4): the jobs `SCHEDULE.toml` starts on a clock and the
  // sources `WATCH.toml` listens to, each in the order its file was
  // written, and a file that did not read with its reason. Both files
  // are edited by hand at the city's root, so this group shows them and
  // writes nothing: a form here would be a second writer of a grammar
  // the city alone reads.

  import { QUERIES } from "../../core/asking";
  import { fill, say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { Cadence } from "../../wire";
  import EmptyState from "../parts/empty.svelte";
  import Unanswered from "../parts/unanswered.svelte";

  const u = ui();
  const { lang } = u;
  const asked = u.conn.asking.ask(QUERIES.automation);
  const answer = $derived($asked !== undefined && "automation" in $asked ? $asked.automation : null);

  const uid = $props.id();
  const MINUTES_A_DAY = 1440;

  function clock(minute: number): string {
    const hour = Math.floor(minute / 60) % 24;
    return `${String(hour).padStart(2, "0")}:${String(minute % 60).padStart(2, "0")}`;
  }

  // A week's minute counts from Monday 00:00 UTC (`city::schedule`); the
  // day's name is the browser's own word for it in the page's language.
  function weekday(minute: number): string {
    const day = Math.floor(minute / MINUTES_A_DAY);
    return new Intl.DateTimeFormat($lang, { weekday: "short", timeZone: "UTC" }).format(Date.UTC(2024, 0, 1 + day));
  }

  function cadence(each: Cadence): string {
    if ("every_minutes" in each) return fill(say($lang, "automation_every"), { n: String(each.every_minutes.minutes) });
    if ("daily_at" in each) return fill(say($lang, "automation_daily"), { at: clock(each.daily_at.minute) });
    return fill(say($lang, "automation_weekly"), { day: weekday(each.weekly_at.minute), at: clock(each.weekly_at.minute % MINUTES_A_DAY) });
  }

  const ROW = "grid grid-cols-[minmax(0,10rem)_minmax(0,1fr)] gap-x-base gap-y-hair border-b border-edge py-snug";
</script>

{#if answer === null}
  {#if $asked !== undefined && "unavailable" in $asked}
    <Unanswered query={$asked.unavailable.query} asked={QUERIES.automation} />
  {/if}
{:else}
  <div class="flex flex-col gap-wide">
    {#each answer.unreadable as file (file)}
      <p class="asks rounded-card bg-raised px-base py-snug font-mono text-note text-text-quiet">{file}</p>
    {/each}
    <section class="flex flex-col" aria-labelledby={`${uid}-jobs`}>
      <h3 id={`${uid}-jobs`} class="pb-tight text-label font-label text-text">{say($lang, "automation_jobs")}</h3>
      {#each answer.jobs as job (job.name)}
        <div class={ROW}>
          <span class="truncate font-mono text-note text-text">{job.name}</span>
          <span class="truncate text-note text-text-quiet">{`${job.addr} · ${cadence(job.cadence)}`}</span>
          <span></span>
          <span class="text-note text-text-faint">{job.task}</span>
        </div>
      {:else}
        <EmptyState missing="automation_no_jobs" />
      {/each}
    </section>
    <section class="flex flex-col" aria-labelledby={`${uid}-sources`}>
      <h3 id={`${uid}-sources`} class="pb-tight text-label font-label text-text">{say($lang, "automation_sources")}</h3>
      {#each answer.sources as source (source.name)}
        <div class={ROW}>
          <span class="truncate font-mono text-note text-text">{source.name}</span>
          <span class="truncate text-note text-text-quiet">
            {`${source.addr} · ${say($lang, source.starts_work ? "automation_starts_work" : "automation_tells")}`}
          </span>
          <span></span>
          <code class="truncate font-mono text-note text-text-faint">{source.matches}</code>
        </div>
      {:else}
        <EmptyState missing="automation_no_sources" />
      {/each}
    </section>
    <p class="text-note text-text-faint">{say($lang, "automation_by_hand")}</p>
  </div>
{/if}
