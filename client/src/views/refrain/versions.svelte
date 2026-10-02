<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The versions this page held whole (client-SPEC 4-46), and the diff
  // between two of them: opened, saved by the person, or changed in the
  // city, with the draft as one more when there is one. Two rows of
  // choices name the two sides, newest last; the comparison under them
  // is a read-only view from the editor's chunk, built again whenever
  // either side changes.
  import { fill, say } from "../../core/lang";
  import type { Key } from "../../core/lang";
  import { clock } from "../../core/time";
  import { ui } from "../../ui";
  import Empty from "../parts/empty.svelte";
  import type { Held, Session } from "./session.svelte";
  import { short } from "./reading";

  interface Props {
    readonly session: Session;
    readonly label: string;
    readonly phrases: Readonly<Record<string, string>>;
  }

  const { session, label, phrases }: Props = $props();

  const lang = ui().lang;

  const DRAFT = "draft";
  const SOURCE: Record<Held["source"], Key> = {
    opened: "refrain_version_opened",
    saved: "refrain_version_saved",
    moved: "refrain_version_moved",
  };

  // The draft is the editor's text when the reading opens, not as it is
  // typed: a comparison of a moving text would redraw on every key.
  const draft = $derived(
    session.receipt.kind === "clean" || session.receipt.kind === "saved" ? null : (session.editing?.text() ?? null),
  );
  const sides = $derived([
    ...session.versions.map((each) => ({
      value: each.version,
      label: `${short(each.version)} · ${say($lang, SOURCE[each.source])} · ${clock($lang, each.at)}`,
      text: each.text,
    })),
    ...(draft === null ? [] : [{ value: DRAFT, label: say($lang, "refrain_version_draft"), text: draft }]),
  ]);

  let from = $state<string | null>(null);
  let to = $state<string | null>(null);
  const left = $derived(sides.find((each) => each.value === from) ?? sides.at(-2) ?? null);
  const right = $derived(sides.find((each) => each.value === to) ?? sides.at(-1) ?? null);

  function pickFrom(value: string): void {
    from = value;
  }

  function pickTo(value: string): void {
    to = value;
  }

  let host = $state<HTMLDivElement>();

  $effect(() => {
    const parent = host;
    const a = left;
    const b = right;
    if (parent === undefined || a === null || b === null) return;
    let shown: { readonly destroy: () => void } | null = null;
    let gone = false;
    void import("./editing").then(({ openComparison }) => {
      if (!gone) shown = openComparison(parent, { from: a.text, to: b.text }, label, phrases);
    });
    return () => {
      gone = true;
      shown?.destroy();
    };
  });
</script>

{#if sides.length < 2}
  <Empty missing="refrain_versions_one" />
{:else}
  <!-- Two native lists rather than rows of cells: a version's name, where
       it came from and when do not fit an equal cell (frontend-method). -->
  <div class="grid grid-cols-[auto_minmax(0,1fr)] items-center gap-x-base gap-y-snug border-b border-edge px-wide py-snug">
    {#each [{ key: "refrain_from" as const, held: left, set: pickFrom }, { key: "refrain_to" as const, held: right, set: pickTo }] as side (side.key)}
      <span class="text-note text-text-faint">{say($lang, side.key)}</span>
      <select
        class="h-control-sm w-full min-w-0 rounded-control border border-edge-input bg-raised px-snug text-note text-text"
        aria-label={say($lang, side.key)}
        value={side.held?.value ?? ""}
        onchange={(event) => {
          side.set(event.currentTarget.value);
        }}
      >
        {#each sides as each (each.value)}
          <option value={each.value}>{each.label}</option>
        {/each}
      </select>
    {/each}
  </div>
  {#if left !== null && right !== null && left.text === right.text}
    <p class="p-pane text-note text-text-quiet">{fill(say($lang, "refrain_diff_same"), { version: left.label })}</p>
  {:else}
    <div class="refrain-editor min-h-0 flex-1" bind:this={host}></div>
  {/if}
{/if}
