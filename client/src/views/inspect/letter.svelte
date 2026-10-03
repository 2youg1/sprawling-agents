<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // A letter on the right side (roadmap A25, client D73): one card
  // offered on a document, read four ways - the card's own diff, the
  // conversation of the run that sent it, the whole document at the
  // version the card was written against, and the whole document as it
  // stands, with every open card above its text - and a link to that
  // run's page. The diff is one reading among the four, because a
  // stretch read without the text around it, or without what the run
  // was asked and said, does not say what it changes or why. The
  // conversation is the run page's own thread, drawn with no fork and
  // no retry, so it is read-only here as it is there, and only the turns
  // around the line that offered the card, with a link to the whole
  // session (client D91).
  //
  // The heading takes the focus when the letter opens, so the mailbox
  // that stowed for it does not hand the focus to its key, and Escape
  // in here is the inspector's: the way back to the row is
  // `inspect/open.svelte.ts`'s.
  import { onMount } from "svelte";
  import { readable, type Readable } from "svelte/store";

  import { readAnswer } from "../../core/answered";
  import { fill, say } from "../../core/lang";
  import { adopted } from "../../core/belief/adopted";
  import { buildingOf, toFragment } from "../../core/route";
  import { ui } from "../../ui";
  import type { Address, Answer, B3Hash, RunSummary } from "../../wire";
  import Segmented from "../parts/segmented.svelte";
  import Card from "../refrain/proposals_card.svelte";
  import RefRain from "../refrain/refrain.svelte";
  import Thread from "../talk/thread.svelte";

  interface Props {
    readonly doc: Address;
    readonly card: B3Hash;
    // The reading the letter opens on; the diff unless a caller asks.
    readonly opening?: Reading;
  }

  const { doc, card, opening = "diff" }: Props = $props();

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;

  type Reading = "diff" | "talk" | "before" | "now";
  // The prop is only where the letter starts; the segmented control moves it after.
  // svelte-ignore state_referenced_locally
  let reading = $state<Reading>(opening);
  let heading = $state<HTMLElement | undefined>(undefined);
  onMount(() => {
    heading?.focus();
  });

  const asked = $derived(u.conn.asking.ask({ proposals: doc }));
  const read = $derived(readAnswer($asked, (answer) => ("proposals" in answer ? answer.proposals : undefined)));
  const held = $derived(read.kind === "held" ? (read.value.open.find((each) => each.id === card) ?? null) : null);
  const building = $derived(buildingOf(doc));
  const path = $derived(doc.slice(building.length + 1));

  // The sender as the run page reads it: the city's summary adopted
  // over what this page folded, so a run the page never saw still draws.
  const NOTHING: Readable<Answer | undefined> = readable(undefined);
  const sender = $derived(held === null ? NOTHING : u.conn.asking.ask({ run_view: { run: held.run } }));
  const sent = $derived.by(() => {
    if (held === null) return undefined;
    const answer = $sender;
    const summary: RunSummary | null = answer !== undefined && "run" in answer ? answer.run : null;
    const mine = $belief.runs[held.run];
    return summary === null ? mine : adopted(summary, mine);
  });
</script>

<div class="flex min-h-0 flex-1 flex-col bg-page">
  <div class="flex flex-wrap items-center gap-x-base gap-y-tight border-b border-edge px-wide py-snug">
    <h2 bind:this={heading} tabindex="-1" class="min-w-0 flex-1 truncate font-mono text-note text-text-quiet" title={doc}>{doc}</h2>
    {#if held !== null}
      <a class="text-note text-text-quiet underline hover:text-text" href={toFragment({ kind: "run", run: held.run })}>
        {fill(say($lang, "letter_sender"), { who: $belief.runs[held.run]?.addr ?? held.run.slice(0, 8) })}
      </a>
    {/if}
    <Segmented
      label={say($lang, "letter_reading")}
      options={[
        { value: "diff", label: say($lang, "letter_diff") },
        { value: "talk", label: say($lang, "letter_talk") },
        { value: "before", label: say($lang, "letter_before") },
        { value: "now", label: say($lang, "letter_now") },
      ]}
      held={reading}
      onPick={(value) => {
        reading = value;
      }}
    />
  </div>
  {#if read.kind !== "held"}
    <p class="px-wide py-snug text-note text-text-faint">…</p>
  {:else if held === null}
    <p class="px-wide py-snug text-note text-text-faint">{say($lang, "letter_gone")}</p>
  {:else if reading === "diff"}
    <div class="min-h-0 flex-1 overflow-auto px-wide py-snug">
      <Card {doc} card={held} version={read.value.version ?? null} />
    </div>
  {:else if reading === "talk"}
    <div class="min-h-0 flex-1 overflow-auto px-wide py-snug">
      {#if sent === undefined}
        <p class="text-note text-text-faint">…</p>
      {:else}
        <Thread run={sent} around={held.offered ?? null} />
      {/if}
    </div>
  {:else}
    <div class="min-h-0 flex-1 overflow-auto">
      {#key reading}
        <RefRain {building} {path} version={reading === "before" ? held.baseline : null} />
      {/key}
    </div>
  {/if}
</div>
