<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // One reminder's checkpoint (docs/frontend-method.md §7J): a dot at its
  // percent of the window, ringed in the page colour so it reads over the
  // line it sits on, green for the first reminder and red for the
  // handoff. The context ring and a sessions row's context bar both draw
  // it here, so the two cannot come to draw a checkpoint two ways; only
  // where it stands differs - round the ring, or along the bar.
  //
  // Drawing only: the reading it belongs to is said in words by the
  // element that owns it.
  import type { Checkpoint } from "./gauge";

  interface Props extends Checkpoint {
    // Round the context ring, turned from twelve o'clock; or along a
    // flat bar, from its left end.
    readonly along: "ring" | "line";
  }

  const { reminder, at, along }: Props = $props();
</script>

<i class={["checkpoint", reminder, along]} style:--at={at} aria-hidden="true"></i>

<style>
  .checkpoint {
    position: absolute;
    width: var(--spacing-checkpoint);
    height: var(--spacing-checkpoint);
    border-radius: var(--radius-pill);
    box-shadow: 0 0 0 var(--spacing-checkpoint-halo) var(--color-page);
    pointer-events: none;
  }
  .first {
    background-color: var(--color-reminder-first);
  }
  .second {
    background-color: var(--color-reminder-second);
  }
  /* The ring's stroke is drawn at 18 of the 40 units of its view box
   * (`gauge.look.svelte`), which scales to the key's square, so the dot
   * is turned about the centre and pushed out by the same share. */
  .ring {
    left: 50%;
    top: 50%;
    translate: -50% -50%;
    transform: rotate(calc(var(--at) * 3.6deg)) translateY(calc(var(--spacing-key) * -0.45));
  }
  .line {
    left: calc(var(--at) * 1%);
    top: 50%;
    translate: -50% -50%;
  }
  /* Forced colours replace both green and red, so the two dots are
   * drawn in the system ink and told apart by where they sit. */
  @media (forced-colors: active) {
    .checkpoint {
      background-color: CanvasText;
    }
  }
</style>
