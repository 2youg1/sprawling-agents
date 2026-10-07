<script lang="ts">
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// How a skeleton is drawn, and nothing else (`./skeleton`,
// `SkeletonLook`). Each bar is the theme's `shimmer`, which stops with
// every other movement when motion is off.

import type { SkeletonLook, Width } from "./skeleton";

const look: SkeletonLook = $props();

const WIDTH: Record<Width, string> = {
  whole: "w-full",
  "five-sixths": "w-5/6",
  "two-thirds": "w-2/3",
  "three-quarters": "w-3/4",
};
</script>

<div {...look.block} class="flex w-full flex-col gap-snug">
  {#each look.bars as bar (bar.slot)}
    <div {...bar.wire} class={["shimmer rounded-control", look.tall ? "h-step" : "h-base", WIDTH[bar.width]]}></div>
  {/each}
</div>
