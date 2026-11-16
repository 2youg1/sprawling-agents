<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  
  
  import type { CardLook } from "./proposals_card";

  const { lead, stale, region, status, body }: CardLook = $props();
</script>

<div class="flex min-w-0 flex-col gap-snug">
  {@render lead?.()}
  {#if stale !== undefined}
    <p class="text-note text-alert">{stale}</p>
  {/if}
  <p {...region} class="text-note empty:hidden">
    {#if status !== undefined}<span
        class={status.tone === "alert" ? "text-alert" : "text-text-faint"}>{status.text}</span
      >{/if}
  </p>
  {#if body.kind === "edit"}
    <ol class="flex flex-col gap-snug">
      {#each body.rows as row (row.key)}
        <li class="flex min-w-0 items-start gap-snug">
          <span class="box">
            {#if row.kind !== "kept"}<input class="size-glyph-sm" {...row.take} />{/if}
          </span>
          {#if row.kind === "kept"}
            <span class="sentence text-text-faint">{row.text}</span>
          {:else if row.kind === "struck"}
            <del class={["sentence removed", !row.taken && "untaken"]}>{row.text}</del>
          {:else}
            <textarea class={["sentence added field", !row.taken && "untaken"]} {...row.words}
            ></textarea>
          {/if}
        </li>
      {/each}
    </ol>
  {:else}
    
    <p class="run">
      {#each body.pieces as piece (piece.key)}{#if piece.kind === "same"}<span
            class="text-text-quiet">{piece.lead}{piece.text}{piece.trail}</span
          >{:else if piece.kind === "delete"}{piece.lead}<del class="removed"
            ><span class="sr-only">{piece.said}</span>{piece.text}</del
          >{piece.trail}{:else}{piece.lead}<ins class={["added", piece.abuts && "abuts"]}
            ><span class="sr-only">{piece.said}</span>{piece.text}</ins
          >{piece.trail}{/if}{/each}
    </p>
  {/if}
</div>

<style>
  
  .sentence {
    flex: 1;
    min-width: 0;
    border: 1px solid transparent;
    border-radius: var(--radius-control);
    padding: var(--spacing-tight) var(--spacing-snug);
    transition: opacity var(--transition-duration-short) var(--ease-arrive);
  }

  
  .box {
    display: flex;
    flex: none;
    align-items: center;
    box-sizing: content-box;
    width: var(--spacing-glyph-sm);
    height: 1lh;
    padding-block: var(--spacing-tight);
    border-block: 1px solid transparent;
  }

  .box input {
    accent-color: var(--color-accent);
  }

  .removed {
    background-color: color-mix(in oklch, var(--color-alert) 12%, transparent);
    color: var(--color-text);
    text-decoration-color: var(--color-alert);
  }

  .added {
    background-color: color-mix(in oklch, var(--color-accent) 12%, transparent);
    color: var(--color-text);
    text-decoration: none;
  }

  .field {
    border-color: var(--color-edge-input);
    field-sizing: content;
    resize: none;
  }

  
  .untaken {
    opacity: 0.6;
    transition-timing-function: var(--ease-leave);
  }

  .run {
    max-height: 16lh;
    overflow-y: auto;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .abuts {
    margin-inline-start: 0.5ch;
  }
</style>
