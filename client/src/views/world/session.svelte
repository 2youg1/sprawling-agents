<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The chosen session, the panorama workbench's middle pane (client/Spec.lean
  // §7K): one line saying where it works and which commit it started
  // from, with the way to its run page, then the instrument sheet and the
  // timeline. Its name and its state are not written here: the sessions
  // column's chosen row names it and draws its state, and the box under
  // the pane carries its address (§7K, "one place on a screen").
  //
  // The session is `chosen.svelte.ts`'s answer - a picked commit's run, or
  // the room's newest - and this pane asks that run's rounds once for the
  // sheet and the timeline both, so the two cannot be reading different
  // turns. The same question is the one the conversation asks, and
  // `asking` merges them by content.
  import type { Snippet } from "svelte";

  import { readAnswer } from "../../core/answered";
  import { fill, say } from "../../core/lang";
  import { toFragment } from "../../core/route";
  import { ui } from "../../ui";
  import type { Address, RoundsAnswer, RunId } from "../../wire";
  import Empty from "../parts/empty.svelte";
  import { commitsIn, pickedCommit, sessionRun } from "./chosen.svelte";
  import Sheet from "./sheet.svelte";
  import Timeline from "./timeline.svelte";

  interface Props {
    readonly here: Address;
    // The run of an earlier session in main, which this pane speaks for
    // instead of the room's (`null`: that session's runs are not held).
    readonly shown?: RunId | null | undefined;
    // The session's name, given only on one column, where the sessions
    // column is another tab and the band under the sheet draws no
    // address, so this pane is the one place on that screen naming it.
    readonly title?: string | undefined;
    // The pane's label: a menu that moves it, in the panorama tier.
    readonly head: Snippet;
  }

  const { here, shown, title, head }: Props = $props();

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;

  const run = $derived(shown === undefined ? sessionRun($belief, here) : shown);
  const picked = $derived(pickedCommit(here));

  let rounds = $state<RoundsAnswer | undefined>(undefined);
  $effect(() => {
    const at = run;
    rounds = undefined;
    if (at === null) return;
    return u.conn.asking.ask({ rounds: { run: at } }).subscribe((held) => {
      rounds = held !== undefined && "rounds" in held ? held.rounds : undefined;
    });
  });

  const commits = $derived(u.conn.asking.ask(commitsIn(here)));
  const held = $derived(readAnswer($commits, (answer) => ("commits" in answer ? answer.commits.commits : undefined)));

  // Where the session works: the worktree it opened, and the commit its
  // changes are measured from.
  const worktree = $derived(rounds?.worktree ?? null);
  const base = $derived(rounds?.opened_at ?? null);
</script>

<section class="flex min-h-0 flex-1 flex-col overflow-hidden" aria-label={say($lang, "world_session")}>
  {@render head()}
  {#if title !== undefined}
    <h3 class="truncate text-heading font-heading text-text">{title}</h3>
  {/if}
  <!-- Where the session works and what it started from: the worktree
  and the base commit, each left out until the city has said it. -->
  <div class="flex shrink-0 items-baseline justify-between gap-pane">
    <p class="flex min-w-0 items-baseline gap-snug text-note text-text-quiet">
      <!-- A long worktree name gives way; the base commit, seven
      characters, stays whole. -->
      {#if worktree !== null}
        <span class="min-w-0 truncate">{fill(say($lang, "world_worktree"), { name: worktree })}</span>
      {/if}
      {#if worktree !== null && base !== null}
        <!-- wording-ok: a separator glyph between two facts, hidden from screen readers -->
        <span aria-hidden="true">·</span>
      {/if}
      {#if base !== null}
        <span class="shrink-0">{fill(say($lang, "world_based_on"), { oid: base.slice(0, 7) })}</span>
      {/if}
    </p>
    {#if run !== null}
      <a class="shrink-0 text-note text-accent hover:text-accent-hover" href={toFragment({ kind: "run", run })}>{say($lang, "world_open_run")}</a>
    {/if}
  </div>
  {#if run === null}
    <div class="mt-snug border-t border-edge">
      <Empty missing="world_no_session" seat="inset" />
    </div>
  {:else}
    <div class="mt-snug flex min-h-0 flex-1 flex-col">
      <Sheet {here} {run} {rounds} />
      <Timeline {run} turns={rounds?.turns ?? []} commits={held.kind === "held" ? held.value : []} {picked} />
    </div>
  {/if}
</section>
