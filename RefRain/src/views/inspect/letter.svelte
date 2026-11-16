<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  import { onMount } from "svelte";
  import { readable, type Readable } from "svelte/store";

  import { readAnswer } from "../../core/answered";
  import { fill, say } from "../../core/lang";
  import { adopted } from "../../core/belief/adopted";
  import { buildingOf, toFragment } from "../../core/route";
  import { ui } from "../../ui";
  import type { Address, Answer, B3Hash, RunSummary } from "../../wire";
  import Segmented from "../parts/segmented.svelte";
  import Card from "../editor/proposals_card.svelte";
  import RefRain from "../editor/refrain.svelte";
  import Thread from "../talk/thread.svelte";
  import TextKey from "./text_key.look.svelte";

  interface Props {
    readonly doc: Address;
    readonly card: B3Hash;

    readonly opening?: Reading;
  }

  const { doc, card, opening = "diff" }: Props = $props();

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;

  type Reading = "diff" | "talk" | "before" | "now";

  // svelte-ignore state_referenced_locally
  let reading = $state<Reading>(opening);
  let heading = $state<HTMLElement | undefined>(undefined);
  onMount(() => {
    heading?.focus();
  });

  const asked = $derived(u.conn.asking.ask({ proposals: doc }));
  const read = $derived(
    readAnswer($asked, (answer) => ("proposals" in answer ? answer.proposals : undefined)),
  );
  const held = $derived(
    read.kind === "held" ? (read.value.open.find((each) => each.id === card) ?? null) : null,
  );
  const building = $derived(buildingOf(doc));
  const path = $derived(doc.slice(building.length + 1));

  const NOTHING: Readable<Answer | undefined> = readable(undefined);
  const sender = $derived(
    held === null ? NOTHING : u.conn.asking.ask({ run_view: { run: held.run } }),
  );
  const sent = $derived.by(() => {
    if (held === null) return undefined;
    const answer = $sender;
    const summary: RunSummary | null = answer !== undefined && "run" in answer ? answer.run : null;
    const mine = $belief.runs[held.run];
    return summary === null ? mine : adopted(summary, mine);
  });
</script>

<div class="flex min-h-0 flex-1 flex-col bg-page">
  <div
    class="flex flex-wrap items-center gap-x-base gap-y-tight border-b border-edge px-wide py-snug"
  >
    <h2
      bind:this={heading}
      tabindex="-1"
      class="min-w-0 flex-1 font-mono text-note break-all text-text-quiet"
    >
      {doc}
    </h2>
    {#if held !== null}
      <TextKey
        label={fill(say($lang, "letter_sender"), {
          who: $belief.runs[held.run]?.addr ?? held.run.slice(0, 8),
        })}
        wire={{ href: toFragment({ kind: "run", run: held.run }) }}
      />
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
