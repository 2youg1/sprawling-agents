<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // How the inspector's small worded key is drawn, and nothing else
  // (client D95): quiet words on the head row that take a raised fill
  // under the pointer, and keep it while a toggle is down. The link or
  // button it is arrives in the wire bag, spread unchanged.
  import type { TextKeyLook } from "./text_key";

  const look: TextKeyLook = $props();

  const SHAPE =
    "text-key flex h-control-sm shrink-0 items-center rounded-control px-snug text-note text-text-faint hover:bg-raised hover:text-text aria-pressed:bg-raised aria-pressed:text-text";
</script>

{#if "href" in look.wire}
  <a {...look.wire} class={SHAPE}>{look.label}</a>
{:else}
  <button {...look.wire} class={SHAPE}>{look.label}</button>
{/if}

<style>
  /* The resting key leaves, the hovered key arrives
   * (docs/frontend-method.md §4-43). */
  .text-key {
    transition:
      color var(--transition-duration-short) var(--ease-leave),
      background-color var(--transition-duration-short) var(--ease-leave);
  }
  .text-key:hover {
    transition-timing-function: var(--ease-arrive);
  }

  /* A forced-colour mode drops the raised fill, so a toggle that is
   * down keeps a rule round it instead. */
  @media (forced-colors: active) {
    .text-key[aria-pressed="true"] {
      outline: 1px solid CanvasText;
    }
  }
</style>
