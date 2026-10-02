<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The chosen session, the panorama workbench's middle pane (client/Spec.lean
  // §7K): its name and state, one line saying where it works and which
  // commit it started from, the instrument sheet, and the timeline.
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
  import type { Address, RoundsAnswer } from "../../wire";
  import Empty from "../parts/empty.svelte";
  import { commitsIn, pickedCommit, sessionRun } from "./chosen.svelte";
  import Sheet from "./sheet.svelte";
  import Timeline from "./timeline.svelte";

  interface Props {
    readonly here: Address;
    // The room's name as the conversation calls it.
    readonly title: string;
    // The pane's label: a menu that moves it, in the panorama tier.
    readonly head: Snippet;
  }

  const { here, title, head }: Props = $props();

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;

  const run = $derived(sessionRun($belief, here));
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

  type State = "running" | "waiting" | "frozen" | "unknown";

  const posture = $derived.by((): State => {
    const doing = run === null ? undefined : $belief.runs[run]?.doing;
    if (doing === undefined) return rounds?.closing === undefined || rounds.closing === null ? "unknown" : "frozen";
    switch (doing.kind) {
      case "frozen":
        return "frozen";
      case "waiting":
        return "waiting";
      case "unknown":
      case "thinking":
      case "calling":
        return "running";
    }
  });
</script>

<section class="flex min-h-0 flex-1 flex-col overflow-hidden" aria-label={say($lang, "world_session")}>
  {@render head()}
  <div class="flex shrink-0 items-baseline justify-between gap-pane">
    <h3 class="truncate text-heading font-heading text-text">{title}</h3>
    <span class="flex shrink-0 items-center gap-snug text-note text-text-quiet">
      {#if posture === "running"}
        <span class="size-dot animate-pulse rounded-pill bg-accent" aria-hidden="true"></span>
        {say($lang, "world_running")}
      {:else if posture === "waiting"}
        <span class="size-dot rounded-pill bg-alert" aria-hidden="true"></span>
        {say($lang, "world_waiting")}
      {:else if posture === "frozen"}
        {say($lang, "world_frozen")}
      {/if}
      {#if run !== null}
        <a class="text-accent hover:text-accent-hover" href={toFragment({ kind: "run", run })}>{say($lang, "world_open_run")}</a>
      {/if}
    </span>
  </div>
  <p class="shrink-0 truncate text-note text-text-faint">
    <span class="text-text-quiet">{here}</span>
    {#if worktree !== null}
      · {fill(say($lang, "world_worktree"), { name: worktree })}
    {/if}
    {#if base !== null}
      · {fill(say($lang, "world_based_on"), { oid: base.slice(0, 7) })}
    {/if}
  </p>
  {#if run === null}
    <div class="mt-snug border-t border-edge">
      <Empty missing="world_no_session" seat="inset" />
    </div>
  {:else}
    <div class="mt-snug flex min-h-0 flex-1 flex-col">
      <Sheet {here} {run} {rounds} commits={held.kind === "held" ? held.value : []} />
      <Timeline {run} turns={rounds?.turns ?? []} commits={held.kind === "held" ? held.value : []} {picked} />
    </div>
  {/if}
</section>
