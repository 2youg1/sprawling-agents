<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // How the line between the editor and the terminal is drawn, and
  // nothing else (client D95): one baseline step of chrome between two
  // rules, with a short grip in its middle that brightens under the
  // pointer and takes the accent under the keyboard's focus. The role,
  // values and handlers arrive in the wire bag spread on the line.
  //
  // The grip is centred by its own box rather than placed a counted
  // number of pixels down, so it stays in the middle at any density.
  import type { SplitLook } from "./split";

  const look: SplitLook = $props();
</script>

<!-- A focusable separator is the APG Window Splitter, a widget in
WAI-ARIA 1.2; its role and handlers arrive in the wire bag. -->
<div {...look.wire} class="split relative shrink-0 cursor-row-resize touch-none border-y border-edge bg-chrome"></div>

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

  /* The resting grip leaves, the hovered and focused grip arrives
   * (docs/frontend-method.md §4-43). */
  .split:hover::after {
    background-color: var(--color-edge-input);
    transition-timing-function: var(--ease-arrive);
  }

  .split:focus-visible::after {
    background-color: var(--color-accent);
    transition-timing-function: var(--ease-arrive);
  }

  /* A forced-colour mode paints the grip in the system's text ink, and
   * its focus in the system's selection colour. */
  @media (forced-colors: active) {
    .split::after {
      background-color: CanvasText;
    }
    .split:focus-visible::after {
      background-color: Highlight;
    }
  }
</style>
