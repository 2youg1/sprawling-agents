<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // How one file's patch is drawn, and nothing else (client D95): every
  // row one baseline step of three high, the code rows on a grid of a
  // number column, a sign column and the line, added and removed lines
  // on the accent and alert washes, and the chosen line marked by a
  // 2 px accent bar at its left edge. A number that quotes its line
  // arrives as a wire bag and is drawn as a link; one that does not is
  // plain ink.
  import Inked from "../parts/inked.svelte";

  import type { PatchLook, PatchRow } from "./patch";

  const look: PatchLook = $props();

  type Tone = Extract<PatchRow, { readonly kind: "code" }>["tone"];

  // Spelled whole so Tailwind reads them out of this file as text.
  const WASH: Record<Tone, string> = {
    added: "bg-accent/12",
    removed: "bg-alert/12",
    context: "",
  };
  const MARK: Record<Tone, string> = {
    added: "text-accent",
    removed: "text-alert",
    context: "text-text-faint",
  };
</script>

<div class="patch w-max min-w-full py-snug font-mono text-note">
  {#each look.rows as row (row.key)}
    {#if row.kind === "withheld"}
      <div class="lead text-text-faint">{row.text}</div>
    {:else if row.kind === "head"}
      <div class="lead whitespace-pre text-text-faint">{row.text}</div>
    {:else if row.kind === "hunk"}
      <div class="lead my-tight flex gap-base bg-raised pr-wide text-text-faint">
        {#if row.folded !== undefined}
          <span class="shrink-0">{row.folded}</span>
        {/if}
        <span class="whitespace-pre">{row.text}</span>
      </div>
    {:else}
      <div class={["code grid pr-wide", WASH[row.tone], row.chosen && "chosen"]} data-line={row.key}>
        {#if row.quote === undefined}
          <span class="pr-pane text-right select-none {MARK[row.tone]}">{row.place}</span>
        {:else}
          <a {...row.quote} class="number pr-pane text-right select-none hover:text-text {MARK[row.tone]}">{row.place}</a>
        {/if}
        <!-- wording-ok: the diff format's own marks, not words. -->
        <span class="select-none {MARK[row.tone]}" aria-hidden="true">{row.sign}</span>
        <span class="whitespace-pre text-text"><Inked text={row.text} source={look.source} /></span>
      </div>
    {/if}
  {/each}
</div>

<style>
  .patch {
    line-height: calc(3 * var(--spacing-baseline));
  }

  /* Rows that are not code start where the code does, past the number
   * and sign columns. */
  .lead {
    padding-left: calc(9 * var(--spacing-baseline));
  }

  .code {
    grid-template-columns: calc(7 * var(--spacing-baseline)) calc(2 * var(--spacing-baseline)) auto;
  }

  .chosen {
    box-shadow: inset var(--spacing-hair) 0 0 var(--color-accent);
  }

  /* The resting number leaves, the hovered number arrives
   * (docs/frontend-method.md §4-43). */
  .number {
    transition: color var(--transition-duration-short) var(--ease-leave);
  }
  .number:hover {
    transition-timing-function: var(--ease-arrive);
  }

  /* A forced-colour mode drops both the washes and the inset bar, so the
   * chosen line keeps a rule at its left edge in the system's selection
   * colour. */
  @media (forced-colors: active) {
    .chosen {
      border-left: var(--spacing-hair) solid Highlight;
    }
  }
</style>
