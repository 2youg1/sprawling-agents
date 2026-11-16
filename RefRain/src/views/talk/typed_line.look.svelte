<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  interface Props {
    readonly typed: number;
    readonly lit: boolean;
  }

  const { typed, lit }: Props = $props();
</script>

<span
  class="typed-line"
  style:--typed="{String(typed)}px"
  data-lit={lit ? "" : undefined}
  aria-hidden="true"
></span>

<style>
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
