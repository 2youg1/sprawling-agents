<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // How the line under the composer's words is painted, and nothing else
  // (docs/frontend-method.md §7I): it runs as far as the words do and
  // fades out after them, so an empty box shows only a short lead from
  // the left edge, the way a terminal shows its prompt, and the first
  // words draw it out like a progress bar. How far the words reach is the
  // seat's measurement (`./typed_line.svelte`).
  interface Props {
    // Pixels from the left edge to the end of the longest line written.
    readonly typed: number;
    readonly lit: boolean;
  }

  const { typed, lit }: Props = $props();
</script>

<span class="typed-line" style:--typed="{String(typed)}px" data-lit={lit ? "" : undefined} aria-hidden="true"></span>

<style>
  /* Registering `--typed` as a length is what lets the gradient move
   * rather than jump when the words grow or the box is emptied. */
  @property --typed {
    syntax: "<length>";
    inherits: false;
    initial-value: 0px;
  }
  .typed-line {
    position: absolute;
    inset-inline: 0;
    bottom: 0;
    height: 1px;
    background: linear-gradient(
      to right,
      var(--color-text-faint) 0,
      var(--color-text-faint) var(--typed),
      transparent calc(var(--typed) + var(--spacing-typed-fade))
    );
    opacity: 0.55;
    pointer-events: none;
    transition:
      --typed var(--transition-duration-panel) var(--ease-arrive),
      opacity var(--transition-duration-short) var(--ease-arrive);
  }
  .typed-line[data-lit] {
    opacity: 1;
  }
  /* Forced colours replace the faint ink; the line keeps its fade in the
   * system's own ink, so it still says how far the words reach. */
  @media (forced-colors: active) {
    .typed-line {
      background: linear-gradient(
        to right,
        CanvasText 0,
        CanvasText var(--typed),
        transparent calc(var(--typed) + var(--spacing-typed-fade))
      );
    }
  }
</style>
