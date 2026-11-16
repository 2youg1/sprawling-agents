<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // One control of the remote group's door, the shipped look: a glyph
  // and a visible name on a raised key, its note in a `Tip`. The type,
  // the disabled state and the press arrive in `wire` from the seat
  // (`remote_door.svelte`), spread whole on the button.

  import Glyph from "../parts/glyph.svelte";
  import Tip from "../parts/tip.svelte";
  import type { DoorKeyLook } from "./remote_door";

  const { glyph, label, note, wire }: DoorKeyLook = $props();
</script>

<Tip text={note}>
  {#snippet children(hint: string)}
    <button
      {...wire}
      class="door-key relative flex h-control shrink-0 items-center gap-tight rounded-control bg-raised px-base text-label text-text hover:bg-raised-hover aria-disabled:bg-raised aria-disabled:text-text-disabled"
      aria-describedby={hint}
    >
      <Glyph name={glyph} size="sm" class="shrink-0" />
      {label}
    </button>
  {/snippet}
</Tip>

<style>
  /* The key lights under the pointer and fades back when it leaves: the
   * resting state leaves, the hovered state arrives
   * (docs/frontend-method.md §4-43). Motion off zeroes the duration. */
  .door-key {
    transition:
      background-color var(--transition-duration-short) var(--ease-leave),
      color var(--transition-duration-short) var(--ease-leave);
  }
  .door-key:hover {
    transition-timing-function: var(--ease-arrive);
  }
</style>
