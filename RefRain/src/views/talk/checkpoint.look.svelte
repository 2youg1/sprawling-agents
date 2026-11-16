<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  import type { Checkpoint } from "./gauge";

  interface Props extends Checkpoint {
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

  @media (forced-colors: active) {
    .checkpoint {
      background-color: CanvasText;
    }
  }
</style>
