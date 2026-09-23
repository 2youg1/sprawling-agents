<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts" module>
  // The two marks the skyline carries that are about one run and one
  // goal rather than about the building: a figure at the door for each
  // run still moving, and a flag on the roof where a pursuit stands.
  // The legend beside the drawing names both.
  //
  // One component draws both, chosen by one exhaustive `kind` rather
  // than two components the file cannot both be: the drawing a mark is
  // and what it stands for are one decision, so it arrives as one
  // value. Every element is one the compiler knows is SVG by its name,
  // because the `<svg>` these are drawn into is in another file.

  import type { RunBelief } from "../../core/belief";
  import type { PursuitLine } from "../../wire";

  export type Placed =
    | {
        readonly kind: "figure";
        readonly run: RunBelief;
        readonly x: number;
        readonly y: number;
        // Stagger between several figures at one door, in milliseconds.
        readonly delay: number;
      }
    | { readonly kind: "flag"; readonly x: number; readonly y: number; readonly line: PursuitLine };

  export interface MarkProps {
    readonly placed: Placed;
  }
</script>

<script lang="ts">
  import type { Doing } from "../../core/doing";
  import { say } from "../../core/lang";
  import { ui } from "../../ui";

  const { placed }: MarkProps = $props();

  const { lang } = ui();

  // The figure's paint, said once: a run waiting for a person is the
  // one posture worth interrupting for, and every other posture is the
  // drawing's ordinary ink.
  const tone = $derived(
    placed.kind === "figure" && placed.run.doing.kind === "waiting"
      ? "fill-alert"
      : "fill-drawn-figure",
  );

  // The posture a figure stands in, in words: the drawing encodes it
  // in shapes and the label names it, so the state survives a
  // forced-colours mode and a screen reader alike. Exhaustive over
  // `Doing`, which is why a new posture cannot arrive unlabelled.
  function postureWord(held: Doing): string {
    switch (held.kind) {
      case "unknown":
        return say($lang, "city_at_work");
      case "thinking":
        return say($lang, "run_doing_thinking");
      case "calling":
        return say($lang, "run_doing_calling");
      case "waiting":
        return say($lang, "run_doing_waiting");
      case "frozen":
        return say($lang, "run_doing_frozen");
    }
  }
</script>

{#if placed.kind === "figure"}
  <g
    class="bob"
    style="animation-delay: {placed.delay}ms; transform-origin: {placed.x}px {placed.y}px"
    aria-label="{placed.run.addr ?? ''} · {postureWord(placed.run.doing)}"
  >
    <path d="M{placed.x - 5} {placed.y} q5 -13 10 0 z" class={tone} />
    <circle cx={placed.x} cy={placed.y - 16} r="4.2" class={tone} />
    {#if placed.run.doing.kind === "thinking"}
      <g class="blink">
        <circle cx={placed.x + 7} cy={placed.y - 24} r="1.2" class="fill-drawn-aside" />
        <circle cx={placed.x + 10.5} cy={placed.y - 28} r="1.6" class="fill-drawn-aside" />
        <circle cx={placed.x + 15} cy={placed.y - 33} r="2.2" class="fill-drawn-aside" />
      </g>
    {/if}
    {#if placed.run.doing.kind === "calling"}
      <rect x={placed.x + 5.5} y={placed.y - 11} width="6" height="6" rx="1.2" class="fill-accent" />
    {/if}
    {#if placed.run.doing.kind === "waiting"}
      <circle cx={placed.x} cy={placed.y - 16} r="8" class="fill-none stroke-alert" stroke-width="1.4" />
    {/if}
  </g>
{:else if placed.kind === "flag"}
  <g aria-label={placed.line.goal}>
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
      class={placed.line.state === "running" ? "fill-accent wave" : "fill-drawn-part"}
      style="transform-origin: {placed.x}px {placed.y - 18}px"
    />
  </g>
{/if}
