<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  import Inked from "../parts/inked.svelte";

  import type { PatchLook, PatchRow } from "./patch";

  const look: PatchLook = $props();

  type Tone = Extract<PatchRow, { readonly kind: "code" }>["tone"];

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
      <div
        class={["code grid pr-wide", WASH[row.tone], row.chosen && "chosen"]}
        data-line={row.key}
      >
        {#if row.quote === undefined}
          <span class="pr-pane text-right select-none {MARK[row.tone]}">{row.place}</span>
        {:else}
          <a
            {...row.quote}
            class="number pr-pane text-right select-none hover:text-text {MARK[row.tone]}"
            >{row.place}</a
          >
        {/if}
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

  .lead {
    padding-left: calc(9 * var(--spacing-baseline));
  }

  .code {
    grid-template-columns: calc(7 * var(--spacing-baseline)) calc(2 * var(--spacing-baseline)) auto;
  }

  .chosen {
    box-shadow: inset var(--spacing-hair) 0 0 var(--color-accent);
  }

  .number {
    transition: color var(--transition-duration-short) var(--ease-leave);
  }
  .number:hover {
    transition-timing-function: var(--ease-arrive);
  }

  @media (forced-colors: active) {
    .chosen {
      border-left: var(--spacing-hair) solid Highlight;
    }
  }
</style>
