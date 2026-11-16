<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  import type { Tone } from "./button";

  const WEAR: Record<Tone, string> = {
    primary: "h-control-lg bg-accent text-on-accent data-[state=idle]:hover:bg-accent-hover",
    secondary: "h-control bg-raised text-text data-[state=idle]:hover:bg-raised-hover",
    quiet: "h-control text-text-quiet data-[state=idle]:hover:bg-raised",
    destructive:
      "h-control bg-raised text-alert data-[state=idle]:hover:bg-alert data-[state=idle]:hover:text-on-accent",
  };

  const MUTED = "aria-disabled:bg-raised aria-disabled:text-text-disabled";

  const SHAPE =
    "inline-flex items-center gap-snug rounded-control px-base text-label " +
    "transition-[background-color,color,opacity,transform] ease-leave " +
    "hover:ease-arrive active:ease-arrive active:scale-(--scale-press) " +
    "still:transition-none still:active:scale-100";
</script>

<script lang="ts">
  import type { Snippet } from "svelte";

  import type { ButtonLook } from "./button";
  import Tip from "./tip.svelte";

  const look: ButtonLook = $props();

  const control: Snippet<Parameters<typeof drawControl>> = drawControl;
</script>

{#snippet drawControl(hint: string | undefined)}
  <button {...look.wire(hint)} data-state={look.state} class="{SHAPE} {WEAR[look.tone]} {MUTED}">
    {#if look.state === "loading"}
      <span class="inline-block size-dot pulse rounded-pill bg-current"></span>
    {/if}
    {look.label}
  </button>
{/snippet}

{#if look.why === undefined}
  {@render control(undefined)}
{:else}
  <Tip text={look.why}>
    {#snippet children(hint: string)}
      {@render control(hint)}
    {/snippet}
  </Tip>
{/if}
