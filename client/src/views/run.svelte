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
  // tab-and-panel association (client-SPEC 7-8 item 6).

  import { Option, Schema } from "effect";
  import { SvelteMap } from "svelte/reactivity";
  import { readable } from "svelte/store";
  import type { Readable } from "svelte/store";

  import { readAnswer } from "../core/answered";
  import { adopted } from "../core/belief/adopted";
  import { cancel, steer } from "../core/commands";
  import { sendingInto } from "../core/doing";
  import { fill, say } from "../core/lang";
  import type { Key } from "../core/lang";
  import { buildingOf, roomOf } from "../core/route";
  import { count } from "../core/time";
  import { ui } from "../ui";
  import { Address } from "../wire";
  import type {
    Answer,
    EvidenceKind,
    Query,
    RunId,
    RoundsAnswer,
    RunSummary,
    Turn,
    Used,
  } from "../wire";
  import Changes from "./changes.svelte";
  import Button from "./parts/button.svelte";
  import EmptyState from "./parts/empty.svelte";
  import Path from "./parts/path.svelte";
  import Unanswered from "./parts/unanswered.svelte";
  import Tabs from "./parts/tabs.svelte";
  import type { Lens } from "./parts/tabs.svelte";
  import Head from "./run/head.svelte";
  import Monitor from "./monitor/monitor.svelte";
  import { NO_TAIL } from "../core/live_output";
  import type { Share } from "./run/lanes";
  import Prompt from "./run/prompt.svelte";
  import River from "./run/river.svelte";
  import Composer from "./talk/composer.svelte";
  import Thread from "./talk/thread.svelte";
  import { lastCheckpointIn } from "./checkpoints";

  interface Props {
    readonly run: RunId;
  }

  const { run }: Props = $props();

  type RunLens = "time" | "turns" | "monitor" | "prompt" | "context" | "changes" | "evidence";

  const EVERY: readonly RunLens[] = ["time", "turns", "monitor", "prompt", "context", "changes", "evidence"];
  const WORDS: Record<RunLens, Key> = {
    time: "run_time",
    turns: "run_turns",
    monitor: "run_monitor",
    prompt: "run_prompt",
    context: "run_context",
    changes: "run_changes",
    evidence: "run_evidence",
  };
  const EVIDENCE_WORD: Record<EvidenceKind, Key> = {
    screenshot: "evidence_screenshot",
    finished: "evidence_finished",
  };
  const NOTHING: Readable<Answer | undefined> = readable(undefined);

  // The lens the person chose; until they choose, the run's own state
  // picks one (`firstLens` below).
  let chosen = $state<RunLens | null>(null);
  const lenses = $derived(EVERY.map((id) => ({ id, label: say($lang, WORDS[id]) })));

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
  // (client-SPEC 4-24 third).
  const mine = $derived($belief.runs[run]);
  const shown = $derived(summary === null ? mine : adopted(summary, mine));

  const turns = $derived(rounds?.turns ?? []);
  const checkpoint = $derived(rounds?.opened_at ?? null);
  const lastCheckpoint = $derived(lastCheckpointIn(turns));


  const peak = $derived(peakOf(turns));
  const seen = $derived(seenOf(turns));
  const live = $derived(shown !== undefined && shown.doing.kind !== "frozen");
  const room = $derived(shown?.addr ?? null);
  // Where the changes lens stops. A live run that has not checkpointed past
  // its opening is read against the working tree, which is where its
  // edits are; a closed run stops at its last checkpoint, or at its opening
  // when it never checkpointed, so edits made after it ended are not counted.
  const changedTo = $derived(live ? (lastCheckpoint === checkpoint ? null : lastCheckpoint) : (lastCheckpoint ?? checkpoint));

  // The lens a run opens on (client-SPEC 12-28): a run still going is
  // watched where its time goes; a run that is over and checkpointed
  // past its opening is read for what it changed, which is what a link
  // from a finished run, a commit or `/diff` is followed for; a run that
  // is over and changed nothing is read for what it said.
  const firstLens = $derived.by((): RunLens => {
    if (live) return "time";
    return lastCheckpoint !== null && lastCheckpoint !== checkpoint ? "changes" : "turns";
  });
  const current = $derived(chosen ?? firstLens);

  // The evidence question exists only while its lens is open: a
  // watched answer is refreshed when stale, and nobody is looking at
  // this one between visits.
  const evidenceQuestion = $derived<Query>({ evidence: { run } });
  const evidenceStore = $derived(current === "evidence" ? u.conn.asking.ask(evidenceQuestion) : NOTHING);
  const evidenceRead = $derived(
    readAnswer($evidenceStore, (held) => ("evidence" in held ? held.evidence.items : undefined)),
  );
  const items = $derived(evidenceRead.kind === "held" ? evidenceRead.value : undefined);

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

  function peakOf(round: readonly Turn[]): number {
    return Math.max(1, ...round.map((turn) => (turn.used?.input ?? 0) + (turn.used?.output ?? 0)));
  }

  // What this run read, most consulted first: the window's own list
  // says what it holds, and this says what it kept going back to.
  function seenOf(round: readonly Turn[]): readonly (readonly [string, number])[] {
    const files = new SvelteMap<string, number>();
    for (const turn of round) {
      for (const call of turn.calls) {
        if (
          (call.tool === "read" || call.tool === "search") &&
          call.subject !== null &&
          call.subject !== undefined
        ) {
          files.set(call.subject, (files.get(call.subject) ?? 0) + 1);
        }
      }
    }
    return [...files.entries()].sort((a, b) => b[1] - a[1]);
  }

  function barOf(used: Used | null | undefined, top: number, part: "cached" | "input" | "output"): number {
    if (used === null || used === undefined) return 0;
    const held = Math.min(used.cached, used.input);
    const value = part === "cached" ? held : part === "input" ? used.input - held : used.output;
    return (value / top) * 100;
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
    const picked = EVERY.find((each) => each === id);
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
        <Thread run={shown} who={roomWord(room)} />
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
    <div class="grid grid-cols-[repeat(auto-fit,minmax(320px,1fr))] gap-wide">
      <section>
        <h2 class="mb-base text-note text-text-faint">{say($lang, "run_window")}</h2>
        {#if turns.some((turn) => turn.used !== null && turn.used !== undefined)}
          <ul class="text-note">
            {#each turns as turn (turn.number)}
              <li class="my-tight flex items-center gap-snug">
                <span class="w-figure shrink-0 text-text-faint"
                  >{fill(say($lang, "run_turn_n"), { n: String(turn.number) })}</span
                >
                <!-- The legend under the list names the three colours,
                     so a hint on each segment would say a second time
                     what the page already says once. -->
                <span
                  class="flex h-dot flex-1 overflow-hidden rounded-pill bg-track"
                  aria-hidden="true"
                >
                  <span class="bg-mark" style:width="{barOf(turn.used, peak, 'cached')}%"></span>
                  <span class="bg-accent" style:width="{barOf(turn.used, peak, 'input')}%"></span>
                  <span class="bg-accent-solid" style:width="{barOf(turn.used, peak, 'output')}%"
                  ></span>
                </span>
                <span class="w-figure shrink-0 text-right text-text-faint">
                  {turn.used !== null && turn.used !== undefined
                    ? count(turn.used.input + turn.used.output)
                    : "—"}
                </span>
              </li>
            {/each}
          </ul>
          <p class="mt-snug text-note text-text-faint">
            <span class="mr-base"><span class="inline-block size-dot rounded-pill bg-mark"></span>
              {say($lang, "run_cached")}</span>
            <span class="mr-base"><span class="inline-block size-dot rounded-pill bg-accent"></span>
              {say($lang, "run_input")}</span>
            <span><span class="inline-block size-dot rounded-pill bg-accent-solid"></span>
              {say($lang, "run_output")}</span>
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
                <span class="shrink-0 text-text-faint">{n > 1 ? `×${String(n)}` : ""}</span>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="text-text-faint">{say($lang, "run_read_nothing")}</p>
        {/if}
      </section>
    </div>
  {:else if eye.id === "changes"}
    {#if checkpoint !== null}
      <Changes base={checkpoint} head={changedTo} />
    {:else}
      <EmptyState missing="run_no_checkpoint" />
    {/if}
  {:else if eye.id === "evidence"}
    {#if evidenceRead.kind === "unavailable"}
      <Unanswered query={evidenceRead.query} asked={evidenceQuestion} />
    {:else if items === undefined}
      <p class="text-text-faint">…</p>
    {:else if items.length > 0}
      <ul class="text-note">
        {#each items as item (item.at)}
          <li class="flex items-center gap-base border-b border-edge py-snug">
            <span class="w-figure shrink-0 text-text-faint">{say($lang, EVIDENCE_WORD[item.kind])}</span>
            <span class="flex-1 truncate font-mono text-text-quiet">{item.locator}</span>
            {#if item.picture !== null && item.picture !== undefined}
              <span class="text-text-faint"
                >{item.picture.width}×{item.picture.height} {item.picture.media_type}</span
              >
            {/if}
            <span class="text-text-faint">#{item.at}</span>
          </li>
        {/each}
      </ul>
    {:else}
      <EmptyState missing="run_no_evidence" />
    {/if}
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
      <Button label={say($lang, "run_cancel")} tone="secondary" onPress={() => u.send(cancel(run))} />
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
