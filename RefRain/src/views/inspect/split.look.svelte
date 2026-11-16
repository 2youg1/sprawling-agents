<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  import type { SplitLook } from "./split";

  const look: SplitLook = $props();
</script>

<div
  {...look.wire}
  class="split relative shrink-0 cursor-row-resize touch-none border-y border-edge bg-chrome"
></div>

<style>
  .split {
    height: calc(var(--spacing-baseline) + 1px);
  }

  .split::after {
    content: "";
    position: absolute;
    inset: 0;
    margin: auto;
    width: var(--spacing-section);
    height: var(--spacing-hair);
    border-radius: var(--radius-pill);
    background-color: var(--color-edge-panel);
    transition: background-color var(--transition-duration-short) var(--ease-leave);
  }

  .split:hover::after {
    background-color: var(--color-edge-input);
    transition-timing-function: var(--ease-arrive);
  }

  .split:focus-visible::after {
    background-color: var(--color-accent);
    transition-timing-function: var(--ease-arrive);
  }

  @media (forced-colors: active) {
    .split::after {
      background-color: CanvasText;
    }
    .split:focus-visible::after {
      background-color: Highlight;
    }
  }
</style>
