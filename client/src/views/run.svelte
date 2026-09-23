<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // One run under five lenses: what was said (turns), what it was told
  // before it said anything (prompt), what it has seen and how full its
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

  import { adopted } from "../core/belief";
  import { cancel, steer } from "../core/commands";
  import { sendingInto } from "../core/doing";
  import { fill, say } from "../core/lang";
  import type { Key } from "../core/lang";
  import { buildingOf, roomOf, toFragment } from "../core/route";
  import { count, usd } from "../core/time";
  import { ui } from "../ui";
  import { Address } from "../wire";
  import type {
    Answer,
    EvidenceItem,
    EvidenceKind,
    GitOid,
    RunId,
    RoundsAnswer,
    RunSummary,
    Turn,
    Used,
  } from "../wire";
  import Changes from "./changes.svelte";
  import EmptyState from "./parts/empty.svelte";
  import Path from "./parts/path.svelte";
  import Tabs from "./parts/tabs.svelte";
  import type { Lens } from "./parts/tabs.svelte";
  import Prompt from "./run/prompt.svelte";
  import Composer from "./talk/composer.svelte";
  import Thread from "./talk/thread.svelte";

  interface Props {
    readonly run: RunId;
  }

  const { run }: Props = $props();

  type RunLens = "turns" | "prompt" | "context" | "changes" | "evidence";

  const EVERY: readonly RunLens[] = ["turns", "prompt", "context", "changes", "evidence"];
  const WORDS: Record<RunLens, Key> = {
    turns: "run_turns",
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

  let current = $state<RunLens>("turns");
  const lenses = $derived(EVERY.map((id) => ({ id, label: say($lang, WORDS[id]) })));

  const u = ui();
  const lang = u.lang;
  const belief = u.conn.belief;

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
  const fence = $derived(rounds?.opened_at ?? null);
  const lastFence = $derived.by((): GitOid | null => {
    for (let at = turns.length - 1; at >= 0; at -= 1) {
      for (const note of turns[at]?.notes ?? []) {
        if ("fenced" in note) return note.fenced.oid;
      }
    }
    return null;
  });

  // The evidence question exists only while its lens is open: a
  // watched answer is refreshed when stale, and nobody is looking at
  // this one between visits.
  const evidenceStore = $derived(
    current === "evidence" ? u.conn.asking.ask({ evidence: { run } }) : NOTHING,
  );
  const items = $derived.by((): readonly EvidenceItem[] | undefined => {
    const held = $evidenceStore;
    return held !== undefined && "evidence" in held ? held.evidence.items : undefined;
  });

  const peak = $derived(peakOf(turns));
  const seen = $derived(seenOf(turns));
  const spent = $derived(spentOf(turns));
  const live = $derived(shown !== undefined && shown.doing.kind !== "frozen");
  const room = $derived(shown?.addr ?? null);

  function peakOf(round: readonly Turn[]): number {
    return Math.max(1, ...round.map((turn) => (turn.used?.input ?? 0) + (turn.used?.output ?? 0)));
  }

  function spentOf(round: readonly Turn[]): number {
    return round.reduce((sum, turn) => sum + (turn.spent ?? 0), 0);
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

  function pick(id: string): void {
    const chosen = EVERY.find((each) => each === id);
    if (chosen !== undefined) current = chosen;
  }
</script>

{#snippet panel(eye: Lens)}
  {#if eye.id === "turns"}
    <div class="mx-auto max-w-talk">
      {#if shown !== undefined}
        <Thread run={shown} who={roomWord(room)} />
      {:else}
        <p class="text-text-disabled">…</p>
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
  {:else if eye.id === "prompt"}
    <Prompt {run} />
  {:else if eye.id === "context"}
    <div class="grid grid-cols-[repeat(auto-fit,minmax(320px,1fr))] gap-wide">
      <section>
        <h2 class="mb-base text-label font-label text-text-quiet">{say($lang, "run_window")}</h2>
        {#if turns.some((turn) => turn.used !== null && turn.used !== undefined)}
          <ul class="text-note">
            {#each turns as turn (turn.number)}
              <li class="my-tight flex items-center gap-snug">
                <span class="w-figure shrink-0 text-text-disabled"
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
          <p class="mt-snug text-note text-text-disabled">
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
        {#if spent > 0}
          <p class="mt-base text-note text-text-quiet">
            {fill(say($lang, "run_spent"), { usd: usd(spent) })}
          </p>
        {/if}
      </section>
      <section>
        <h2 class="mb-base text-label font-label text-text-quiet">{say($lang, "run_read_files")}</h2>
        {#if seen.length > 0}
          <ul class="text-note">
            {#each seen as [file, n] (file)}
              <li class="my-tight flex items-center justify-between gap-base">
                <Path path={file} onOpen={opening(file)} />
                <span class="shrink-0 text-text-disabled">{n > 1 ? `×${String(n)}` : ""}</span>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="text-text-faint">{say($lang, "run_read_nothing")}</p>
        {/if}
      </section>
    </div>
  {:else if eye.id === "changes"}
    {#if fence !== null}
      <Changes base={fence} head={lastFence === fence ? null : lastFence} />
    {:else}
      <EmptyState missing="run_no_fence" />
    {/if}
  {:else if eye.id === "evidence"}
    {#if items === undefined}
      <p class="text-text-disabled">…</p>
    {:else if items.length > 0}
      <ul class="text-note">
        {#each items as item (item.at)}
          <li class="flex items-center gap-base border-b border-edge py-snug">
            <span class="w-figure shrink-0 text-text-faint">{say($lang, EVIDENCE_WORD[item.kind])}</span>
            <span class="flex-1 truncate font-mono text-text-quiet">{item.locator}</span>
            {#if item.picture !== null && item.picture !== undefined}
              <span class="text-text-disabled"
                >{item.picture.width}×{item.picture.height} {item.picture.media_type}</span
              >
            {/if}
            <span class="text-text-disabled">#{item.at}</span>
          </li>
        {/each}
      </ul>
    {:else}
      <EmptyState missing="run_no_evidence" />
    {/if}
  {/if}
{/snippet}

<div class="flex min-h-0 w-full max-w-page flex-1 flex-col px-pane pt-wide">
  <div class="flex flex-wrap items-center gap-base pb-base">
    {#if room !== null}
      <a
        href={toFragment({ kind: "building", address: buildingOf(room) })}
        class="text-note text-text-faint">{buildingOf(room)}</a
      >
      <span class="text-text-disabled">/</span>
      <a href={toFragment({ kind: "talk", address: room })} class="text-note text-text-faint"
        >{roomOf(room)}</a
      >
      <span class="text-text-disabled">/</span>
    {/if}
    <!-- The run's own id when nothing has named the task yet. The
         summary's `who` is not offered here: it is the resident, and a
         resident's name standing where the task stands reads as a task
         somebody set. -->
    <h1 tabindex="-1" class="truncate text-heading font-heading">
      {rounds?.opening?.task ?? shown?.task ?? run}
    </h1>
    <span class="flex-1"></span>
    {#if live}
      <button
        type="button"
        class="h-control-sm rounded-control px-base text-label text-text-quiet hover:bg-chrome hover:text-alert"
        onclick={() => u.send(cancel(run))}
      >
        {say($lang, "run_cancel")}
      </button>
    {/if}
  </div>
  {#if rounds?.opening?.goal}
    <p class="mb-base text-note text-text-faint">{say($lang, "run_goal")}: {rounds.opening.goal}</p>
  {/if}
  <Tabs
    label={say($lang, "run_lenses")}
    {lenses}
    current={current}
    onPick={pick}
    {panel}
  />
</div>
