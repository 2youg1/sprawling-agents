<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // How one printed path is drawn, and nothing else: one line in the
  // mono face cut off with an ellipsis at the end, a button when it
  // opens inside the page, and the reveal key beside it. What is shown
  // and what each press does arrive decided (`./path.ts`), so another
  // look draws the same path by taking the same `PathLook`.
  import IconButton from "./icon_button.svelte";
  import type { PathLook } from "./path";

  const look: PathLook = $props();

  const TEXT = "min-w-0 truncate font-mono text-note text-text-quiet";
</script>

<span class="inline-flex min-w-0 max-w-full items-baseline gap-tight">
  {#if look.open === undefined}
    <span class={TEXT}>{look.shown}</span>
  {:else}
    <button
      {...look.open}
      class="{TEXT} text-left underline decoration-edge underline-offset-2 transition-colors ease-leave hover:text-text hover:ease-arrive still:transition-none"
    >
      {look.shown}
    </button>
  {/if}
  <IconButton {...look.reveal} />
</span>
