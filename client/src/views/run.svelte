<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // One run under seven lenses: where its time went (time), what was said (turns), its code
  // beside its terminal (monitor), what it was told before it said anything (prompt), what it has seen and how full its
  // window is (context), what moved on disk (changes), and what it left
  // to be checked (evidence). The conversation is the same component
  // the first page draws, so a run reads the same from both doors.
  //
  // The lens set is `parts/tabs.svelte`, which owns both halves of the
  // tab-and-panel association (client/Spec.lean §7-8 item 6).

  import { adopted } from "../core/belief/adopted";
  import { cancel, steer } from "../core/commands";
  import { sendingInto } from "../core/doing";
  import { fill, say } from "../core/lang";
  import { STOP } from "../core/slash";
  import type { Key } from "../core/lang";
  import { RUN_LENSES, roomOf, type RunLens } from "../core/route";
  import { ui } from "../ui";
  import type { Address, RunId, RoundsAnswer, RunSummary } from "../wire";
  import Changes from "./changes.svelte";
  import Button from "./parts/button.svelte";
  import EmptyState from "./parts/empty.svelte";
  import Tabs from "./parts/tabs.svelte";
  import type { Lens } from "./parts/tabs.svelte";
  import Head from "./run/head.svelte";
  import Monitor from "./monitor/monitor.svelte";
  import { NO_TAIL } from "../core/live_output";
  import type { Share } from "./run/lanes";
  import Context from "./run/context.svelte";
  import Evidence from "./run/evidence.svelte";
  import Prompt from "./run/prompt.svelte";
  import River from "./run/river.svelte";
  import Composer from "./talk/composer.svelte";
  import Thread from "./talk/thread.svelte";
  import { lastCheckpointIn } from "./checkpoints";

  interface Props {
    readonly run: RunId;
    readonly lens?: RunLens | undefined;
  }

  const { run, lens }: Props = $props();

  const WORDS: Record<RunLens, Key> = {
    time: "run_time",
    turns: "run_turns",
    monitor: "run_monitor",
    prompt: "run_prompt",
    context: "run_context",
    changes: "run_changes",
    evidence: "run_evidence",
  };

  // The lens the person chose; until they choose, the run's own state
  // picks one (`firstLens` below).
  // svelte-ignore state_referenced_locally (a link's lens is where the page opens; pages.svelte remounts it for another)
  let chosen = $state<RunLens | null>(lens ?? null);
  const lenses = $derived(RUN_LENSES.map((id) => ({ id, label: say($lang, WORDS[id]) })));

  const u = ui();
  const lang = u.lang;
  const belief = u.conn.belief;
  const tails = u.conn.live;

  const roundsStore = $derived(u.conn.asking.ask({ rounds: { run } }));
  const rounds = $derived.by((): RoundsAnswer | undefined => {
    const held = $roundsStore;
    return held !== undefined && "rounds" in held ? held.rounds : undefined;
  });

  // What the city says this run is, asked of the city rather than
  // folded out of the record. A page opened on `#/run/<id>` after a
  // reload has seen none of the records that made this run, and the
  // city view carries only the runs it still lists - so which room the
  // run belongs to and whether it is over used to be blank on exactly
  // the runs a person reaches by a link somebody sent them.
  const summaryStore = $derived(u.conn.asking.ask({ run_view: { run } }));
  const summary = $derived.by((): RunSummary | null => {
    const held = $summaryStore;
    return held !== undefined && "run" in held ? held.run : null;
  });

  // Two readings of one run, and the later one wins. The stream says
  // what is happening now, the summary says what the city has written
  // down, and both state a ledger position - so which is newer is a
  // comparison rather than a preference, and `core/belief` owns that
  // comparison because the city page folds the same two readings
  // (client/Spec.lean §4-24 third).
  const mine = $derived($belief.runs[run]);
  const shown = $derived(summary === null ? mine : adopted(summary, mine));

  const turns = $derived(rounds?.turns ?? []);
  const checkpoint = $derived(rounds?.opened_at ?? null);
  const lastCheckpoint = $derived(lastCheckpointIn(turns));

  const live = $derived(shown !== undefined && shown.doing.kind !== "frozen");
  const room = $derived(shown?.addr ?? null);
  // Where the changes lens stops. A live run that has not checkpointed past
  // its opening is read against the working tree, which is where its
  // edits are; a closed run stops at its last checkpoint, or at its opening
  // when it never checkpointed, so edits made after it ended are not counted.
  const changedTo = $derived(live ? (lastCheckpoint === checkpoint ? null : lastCheckpoint) : (lastCheckpoint ?? checkpoint));

  // The lens a run opens on (client D28): a run still going is
  // watched where its time goes; a run that is over and checkpointed
  // past its opening is read for what it changed, which is what a link
  // from a finished run, a commit or `/diff` is followed for; a run that
  // is over and changed nothing is read for what it said.
  const firstLens = $derived.by((): RunLens => {
    if (live) return "time";
    return lastCheckpoint !== null && lastCheckpoint !== checkpoint ? "changes" : "turns";
  });
  const current = $derived(chosen ?? firstLens);

  // The run's clock as the page knows it: from the opening (or the
  // first turn) to the closing, or to now while the run is live.
  const from = $derived(rounds?.opening?.at ?? turns[0]?.t ?? shown?.started ?? null);
  const to = $derived.by((): number | null => {
    if (rounds?.closing !== null && rounds?.closing !== undefined) return rounds.closing.at;
    const last = turns.at(-1)?.t ?? from;
    return last === null ? null : live ? Math.max(last, u.now()) : last;
  });
  const tail = $derived.by((): Share | null => {
    switch (shown?.doing.kind) {
      case "calling":
      case "awaiting_reply":
        return "tool";
      case "waiting":
        return "person";
      case "thinking":
        return "model";
      case "frozen":
      case "unknown":
      case undefined:
        return null;
    }
  });

  function roomWord(at: Address | null): string {
    return at === null ? say($lang, "talk_resident") : roomOf(at);
  }

  // A comment on a hunk lands in the composer, unsent: the words wait
  // in this run's draft and the turns lens, which mounts the composer
  // and reads the draft as it mounts, is brought forward.
  function draftSteer(text: string): void {
    u.prefs.setDraft(run, text);
    chosen = "turns";
  }

  function pick(id: string): void {
    const picked = RUN_LENSES.find((each) => each === id);
    if (picked !== undefined) chosen = picked;
  }
</script>

{#snippet panel(eye: Lens)}
  {#if eye.id === "time"}
    {#if from !== null && to !== null && turns.length > 0}
      <River {turns} {from} {to} {tail} />
    {:else}
      <EmptyState missing="run_no_turns" />
    {/if}
  {:else if eye.id === "turns"}
    <div class="mx-auto max-w-talk">
      {#if shown !== undefined}
        <Thread run={shown} />
      {:else}
        <p class="text-text-faint">…</p>
      {/if}
      {#if live}
        <div class="mt-wide">
          <Composer
            placeholder={fill(say($lang, "talk_placeholder_room"), { room: roomWord(room) })}
            sending={sendingInto(shown?.doing)}
            draft={run}
            hearing={u.hearing()}
            onSend={(text: string) => u.send(steer(run, text))}
            onStop={() => u.send(cancel(run))}
          />
        </div>
      {/if}
    </div>
  {:else if eye.id === "monitor"}
    <div class="flex h-[70vh] min-h-0 flex-col rounded-card border border-edge">
      <Monitor {turns} tail={$tails[run] ?? NO_TAIL} {live} onDraft={draftSteer} onSteer={(text: string) => u.send(steer(run, text))} />
    </div>
  {:else if eye.id === "prompt"}
    <Prompt {run} />
  {:else if eye.id === "context"}
    <Context {turns} />
  {:else if eye.id === "changes"}
    {#if checkpoint !== null}
      <Changes base={checkpoint} head={changedTo} />
    {:else}
      <EmptyState missing="run_no_checkpoint" />
    {/if}
  {:else if eye.id === "evidence"}
    <Evidence {run} />
  {/if}
{/snippet}

<div class="flex min-h-0 w-full min-w-0 flex-1 flex-col gap-base px-wide py-wide">
  <div class="flex min-w-0 items-start justify-between gap-wide">
    <div class="flex min-w-0 flex-col gap-tight">
      <!-- The run's own id when nothing has named the task yet. The
           summary's `who` is not offered here: it is the resident, and a
           resident's name standing where the task stands reads as a task
           somebody set. -->
      <h1 tabindex="-1" class="truncate text-title font-title">
        {rounds?.opening?.task ?? shown?.task ?? run}
      </h1>
      {#if rounds?.opening?.goal}
        <p class="text-note text-text-quiet">
          <span class="text-text-faint">{say($lang, "run_goal")}</span>
          {rounds.opening.goal}
        </p>
      {/if}
      <p class="figure truncate text-note text-text-faint">{run}</p>
    </div>
    {#if live}
      <Button label={STOP} tone="secondary" onPress={() => u.send(cancel(run))} />
    {/if}
  </div>
  <Head
    {turns}
    doing={shown?.doing}
    closing={rounds?.closing ?? null}
    dispatchedBy={rounds?.opening?.dispatched_by ?? null}
    {room}
    {from}
    {to}
  />
  <Tabs
    label={say($lang, "run_lenses")}
    {lenses}
    current={current}
    onPick={pick}
    {panel}
  />
</div>
