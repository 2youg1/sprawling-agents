<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // One privacy entry: the shipped look. Markup only; every word, value
  // and callback arrives as `EntryLook` from the seat (`entry.svelte`).
  //
  // What a person could overlook is drawn open, not folded, because it
  // is the part of an entry a person most needs before pressing; the
  // full target path is folded, because it is for the person who wants
  // to check it with another tool. Long tokens (registry paths, the
  // person's own lines) break anywhere rather than run out of the card.

  import Button from "../../parts/button.svelte";
  import type { EntryLook } from "./entry";

  const look: EntryLook = $props();
  const titleId = $derived(`${look.id}-title`);
</script>

<article
  id={look.id}
  tabindex="-1"
  aria-labelledby={titleId}
  class="flex min-w-0 scroll-mt-wide flex-col gap-snug rounded-card bg-raised px-base py-snug"
>
  <h4 id={titleId} class="text-label font-label text-text">{look.title}</h4>
  <p class="text-note text-text-faint [overflow-wrap:anywhere]">
    <span class="text-text-quiet">{look.lines.label}</span>
    {look.lines.text}
  </p>
  <dl class="flex flex-col gap-tight text-note">
    {#each look.notes as note (note.label)}
      <div class="flex flex-col">
        <dt class="text-text-quiet">{note.label}</dt>
        <dd class="text-text">{note.text}</dd>
      </div>
    {/each}
    <div class="flex flex-col rounded-control border border-edge px-snug py-tight">
      <dt class="text-text-quiet">{look.overlooked.label}</dt>
      <dd class="text-text">{look.overlooked.text}</dd>
    </div>
    <div class="flex flex-col">
      <dt class="text-text-quiet">{look.editions.label}</dt>
      <dd class="text-text">{look.editions.text}</dd>
      <dd class="text-text-faint">{look.editionsNote}</dd>
      <dd class={look.fitWeight === "alert" ? "text-alert" : "text-text-quiet"}>{look.fit}</dd>
      {#if look.effect !== null}
        <dd class="text-alert">{look.effect}</dd>
      {/if}
    </div>
  </dl>
  <dl class="grid grid-cols-[auto_minmax(0,1fr)] gap-x-base gap-y-tight text-note">
    {#each look.facts as fact (fact.label)}
      <dt class="text-text-quiet">{fact.label}</dt>
      <dd class={["min-w-0 text-text [overflow-wrap:anywhere]", fact.figure && "font-mono"]}>{fact.value}</dd>
    {/each}
  </dl>
  <details class="text-note">
    <summary class="cursor-pointer text-text-quiet">{look.target.label}</summary>
    <code class="block font-mono text-text [overflow-wrap:anywhere]">{look.target.text}</code>
  </details>
  {#if look.presses.length > 0}
    <div class="flex flex-wrap items-center gap-snug">
      {#each look.presses as press (press.label)}
        <Button label={press.label} tone="secondary" onPress={press.onPress} />
      {/each}
    </div>
  {/if}
  <p role="status" class={["text-note", look.status?.weight === "alert" ? "text-alert" : "text-text-quiet"]}>
    {look.status?.text ?? ""}
  </p>
</article>
