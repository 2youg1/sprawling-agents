<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // A letter on the right side (roadmap A25, client D73): one card
  // offered on a document, read three ways - the card's own diff, the
  // whole document at the version the card was written against, and the
  // whole document as it stands, with every open card above its text -
  // and a link to the run that sent it. The diff is one reading among
  // the three, because a stretch read without the text around it does
  // not say what it changes.
  //
  // The heading takes the focus when the letter opens, so the mailbox
  // that stowed for it does not hand the focus to its key, and Escape
  // in here is the inspector's: the way back to the row is
  // `inspect/open.svelte.ts`'s.
  import { onMount } from "svelte";

  import { readAnswer } from "../../core/answered";
  import { fill, say } from "../../core/lang";
  import { buildingOf, toFragment } from "../../core/route";
  import { ui } from "../../ui";
  import type { Address, B3Hash } from "../../wire";
  import Segmented from "../parts/segmented.svelte";
  import Card from "../refrain/proposals_card.svelte";
  import RefRain from "../refrain/refrain.svelte";

  interface Props {
    readonly doc: Address;
    readonly card: B3Hash;
  }

  const { doc, card }: Props = $props();

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;

  type Reading = "diff" | "before" | "now";
  let reading = $state<Reading>("diff");
  let heading = $state<HTMLElement | undefined>(undefined);
  onMount(() => {
    heading?.focus();
  });

  const asked = $derived(u.conn.asking.ask({ proposals: doc }));
  const read = $derived(readAnswer($asked, (answer) => ("proposals" in answer ? answer.proposals : undefined)));
  const held = $derived(read.kind === "held" ? (read.value.open.find((each) => each.id === card) ?? null) : null);
  const building = $derived(buildingOf(doc));
  const path = $derived(doc.slice(building.length + 1));
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
  {:else}
    <div class="min-h-0 flex-1 overflow-auto">
      {#key reading}
        <RefRain {building} {path} version={reading === "before" ? held.baseline : null} />
      {/key}
    </div>
  {/if}
</div>
