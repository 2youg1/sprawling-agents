<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The two marks the skyline carries that are about one run and one
  // goal rather than about the building: a figure at the door for each
  // run still moving, and a flag on the roof where a pursuit stands.
  // The legend beside the drawing names both, and `./skyline.ts` has
  // already said each mark's words, so this file only draws.
  //
  // One component draws both, chosen by one exhaustive `kind` rather
  // than two components the file cannot both be: the drawing a mark is
  // and what it stands for are one decision, so it arrives as one
  // value. Every element is one the compiler knows is SVG by its name,
  // because the `<svg>` these are drawn into is in another file.
  import type { MarkLook } from "./skyline";

  const { placed }: { readonly placed: MarkLook } = $props();

  // The figure's paint, said once: a run waiting for a person is the
  // one posture worth interrupting for, and every other posture is the
  // drawing's ordinary ink.
  const tone = $derived(placed.kind === "figure" && placed.posture === "waiting" ? "fill-alert" : "fill-drawn-figure");
</script>

{#if placed.kind === "figure"}
  <g
    class="bob"
    style="animation-delay: {placed.delay}ms; transform-origin: {placed.x}px {placed.y}px"
    aria-label={placed.label}
  >
    <path d="M{placed.x - 5} {placed.y} q5 -13 10 0 z" class={tone} />
    <circle cx={placed.x} cy={placed.y - 16} r="4.2" class={tone} />
    {#if placed.posture === "thinking"}
      <g class="blink">
        <circle cx={placed.x + 7} cy={placed.y - 24} r="1.2" class="fill-drawn-aside" />
        <circle cx={placed.x + 10.5} cy={placed.y - 28} r="1.6" class="fill-drawn-aside" />
        <circle cx={placed.x + 15} cy={placed.y - 33} r="2.2" class="fill-drawn-aside" />
      </g>
    {/if}
    {#if placed.posture === "calling"}
      <rect x={placed.x + 5.5} y={placed.y - 11} width="6" height="6" rx="1.2" class="fill-accent" />
    {/if}
    {#if placed.posture === "waiting"}
      <circle cx={placed.x} cy={placed.y - 16} r="8" class="fill-none stroke-alert" stroke-width="1.4" />
    {/if}
  </g>
{:else if placed.kind === "flag"}
  <g aria-label={placed.label}>
    <line
      x1={placed.x}
      y1={placed.y - 22}
      x2={placed.x}
      y2={placed.y}
      class="stroke-drawn-stem"
      stroke-width="1"
    />
    <path
      d="M{placed.x} {placed.y - 22} l16 4 l-16 5 z"
      class={placed.running ? "fill-accent wave" : "fill-drawn-part"}
      style="transform-origin: {placed.x}px {placed.y - 18}px"
    />
  </g>
{/if}
