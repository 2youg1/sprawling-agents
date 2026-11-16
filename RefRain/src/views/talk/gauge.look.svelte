<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  import type { Snippet } from "svelte";

  import Tip from "../parts/tip.svelte";
  import Checkpoint from "./checkpoint.look.svelte";
  import type { RingLook } from "./gauge";

  interface Props {
    readonly ring: RingLook | null;
    readonly children: Snippet;
    readonly onOpen: (() => void) | null;
  }

  const { ring, children: key, onOpen }: Props = $props();
</script>

{#if ring === null}
  <span class="grid size-key shrink-0 place-items-center">{@render key()}</span>
{:else}
  <Tip text={ring.reading}>
    {#snippet children(hint: string)}
      <span class="relative grid size-key shrink-0 place-items-center">
        {#if onOpen !== null}
          <button
            class="absolute inset-0 rounded-pill transition-[background-color,scale] ease-leave hover:bg-raised-hover/40 hover:ease-arrive hover:scale-105 active:scale-(--scale-press) active:ease-arrive still:transition-none still:hover:scale-100 still:active:scale-100"
            {...ring.meter}
            aria-describedby={hint}
            onclick={(event) => {
              if (event.target !== event.currentTarget) return;
              onOpen();
            }}
          ></button>
        {/if}
        <svg
          class="pointer-events-none absolute inset-0 -rotate-90"
          viewBox="0 0 40 40"
          aria-hidden="true"
        >
          <circle
            class="left fill-none stroke-text"
            cx="20"
            cy="20"
            r="18"
            stroke-width="1"
            pathLength="100"
            stroke-dasharray={ring.arc.dasharray}
            stroke-dashoffset={ring.arc.dashoffset}
          />
        </svg>
        {#each ring.checkpoints as checkpoint (checkpoint.reminder)}
          <Checkpoint {...checkpoint} along="ring" />
        {/each}
        <span class="relative z-1 grid place-items-center">{@render key()}</span>
      </span>
    {/snippet}
  </Tip>
{/if}

<style>
  .left {
    transition:
      stroke-dasharray var(--transition-duration-page) var(--ease-arrive),
      stroke-dashoffset var(--transition-duration-page) var(--ease-arrive);
  }
</style>
