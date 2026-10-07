<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // How the city is drawn as a skyline at night, and nothing else: one
  // tower per building along one avenue, City Hall at its centre under
  // a dome, a window per run that lights while the run is going, a
  // figure at the door for each run still moving, a flag on the roof
  // where a pursuit stands, a lamp by the door where something is
  // blocked. Every tower carries its name and how far its plan has got;
  // what the glyphs mean is the legend's job, beside the drawing. Every
  // place, word and tower link arrives in `SkylineLook` (`./skyline.ts`),
  // and each tower's wire bag is spread unchanged on its group.
  //
  // **This file is the outermost drawing component, so every id a
  // gradient needs is declared once in the `defs` below and referenced
  // by name** (client/Spec.lean §4-12). Nothing else in the drawing
  // spells `glow` or `ground` a second time.
  //
  // What changes under the hand or with a run arrives and leaves on the
  // theme's two curves (docs/frontend-method.md §4-43): a tower lights
  // under the pointer or the focus on `arrive` and settles back on
  // `leave`, a window lights on `arrive` and goes dark on `leave`, and a
  // glow fades in behind a tower whose runs start and out when they
  // stop, instead of appearing whole.
  //
  // The drawing is a fixed box with a fixed ratio: a width read from
  // the content and a height read from that width let the scrollbar
  // appear, take width away, shorten the drawing, remove the scrollbar
  // and start again. Nothing here says the city is stopped: the shell's
  // banner says it once, in words.
  import Mark from "./marks.svelte";
  import type { Pane, SkylineLook } from "./skyline";

  const look: SkylineLook = $props();

  // The depth of the plinth a tower stands on, under the ground line.
  const PLINTH = 14;

  // Each window's paint, spelled for Tailwind to read out of this file
  // as text. A window lights on the arrival curve and goes out on the
  // leaving one, because CSS takes the curve from the state it moves to.
  const PANE: Record<Pane, string> = {
    dark: "fill-drawn-hollow ease-leave",
    ended: "fill-drawn-line ease-leave",
    waiting: "fill-alert ease-arrive",
    calling: "fill-accent-solid blink ease-arrive",
    lit: "fill-accent-solid ease-arrive",
  };
</script>

<svg
  viewBox="0 0 {look.width} {look.height}"
  class="block w-full"
  style="aspect-ratio: {look.width} / {look.height}; max-width: {look.width}px"
  role="img"
  aria-label={look.label}
>
  <defs>
    <radialGradient id="glow" cx="50%" cy="60%" r="60%">
      <stop offset="0%" class="[stop-color:var(--color-accent)]" stop-opacity="0.16" />
      <stop offset="100%" class="[stop-color:var(--color-accent)]" stop-opacity="0" />
    </radialGradient>
    <linearGradient id="ground" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%" class="[stop-color:var(--color-drawn-solid)]" />
      <stop offset="100%" class="[stop-color:var(--color-drawn-hollow)]" />
    </linearGradient>
  </defs>
  {#each look.stars as star (star.slot)}
    <circle cx={star.x} cy={star.y} r={star.r} class="fill-drawn-part blink"
      style="animation-delay: {star.delay}ms; animation-duration: 2.6s" />
  {/each}
  <rect x="0" y={look.ground} width={look.width} height={look.height - look.ground} fill="url(#ground)" />
  <line x1="0" y1={look.ground} x2={look.width} y2={look.ground} class="stroke-drawn-line" stroke-width="1" />
  {#each look.towers as tower (tower.key)}
    <g class="group cursor-pointer" {...tower.wire}>
      <rect x={tower.x - 16} y={tower.top - 16} width={tower.w + 32} height={tower.h + 16}
        rx="24" fill="url(#glow)"
        class={["transition-opacity", tower.lit ? "opacity-100 ease-arrive" : "opacity-0 ease-leave"]} />
      {#if tower.hall}
        <path d="M{tower.x + tower.w / 2 - 34} {tower.top} a34 26 0 0 1 68 0 z"
          class="fill-drawn-solid-lit stroke-drawn-edge" stroke-width="1" />
        <line x1={tower.x + tower.w / 2} y1={tower.top - 26} x2={tower.x + tower.w / 2} y2={tower.top - 38}
          class="stroke-drawn-stem" stroke-width="1.2" />
        <circle cx={tower.x + tower.w / 2} cy={tower.top - 40} r="2" class="fill-drawn-aside" />
      {/if}
      <path
        d={tower.outline}
        class={[
          tower.hall ? "fill-drawn-solid-lit" : "fill-drawn-solid",
          tower.picked ? "stroke-accent" : "stroke-drawn-line",
          "transition-colors ease-leave group-hover:fill-drawn-line group-hover:ease-arrive",
          "group-focus-visible:fill-drawn-line group-focus-visible:ease-arrive",
        ]}
        stroke-width={tower.picked ? "2" : "1"}
      />
      <line x1={tower.x + 8} y1={tower.top + 10} x2={tower.x + tower.w - 8} y2={tower.top + 10}
        class="stroke-drawn-line" stroke-width="1" />
      {#each tower.cells as cell (cell.slot)}
        <rect x={cell.x} y={cell.y} width="12" height="14" rx="1.5" class={["transition-colors", PANE[cell.pane]]} />
      {/each}
      {#if tower.hall}
        {#each [0, 1, 2, 3] as column (column)}
          <rect x={tower.x + tower.w / 2 - 42 + column * 28 - 2.5} y={look.ground - 46}
            width="5" height="46" rx="1" class="fill-drawn-line" />
        {/each}
      {/if}
      <rect x={tower.door.x} y={tower.door.y} width={tower.door.w} height={tower.door.h} rx="2" class="fill-drawn-hollow" />
      <rect x={tower.door.x + 2} y={tower.door.y + 2} width={tower.door.w - 4} height="4" rx="1"
        class={["transition-colors", tower.lit ? "fill-accent-solid ease-arrive" : "fill-drawn-solid-lit ease-leave"]} />
      {#if tower.lamp !== null}
        <g class="blink" aria-label={tower.lamp}>
          <line x1={tower.door.x - 10} y1={look.ground} x2={tower.door.x - 10} y2={look.ground - 26}
            class="stroke-drawn-part" stroke-width="1" />
          <circle cx={tower.door.x - 10} cy={look.ground - 28} r="3" class="fill-alert" />
        </g>
      {/if}
      <rect x={tower.x} y={look.ground + 2} width={tower.w} height={PLINTH} rx="2" class="fill-drawn-solid" />
      {#if tower.bar !== null}
        <rect x={tower.x + 6} y={look.ground + 7} width={tower.w - 12} height="3" rx="1.5" class="fill-drawn-line" />
        <rect x={tower.x + 6} y={look.ground + 7} width={(tower.w - 12) * tower.bar.done} height="3" rx="1.5" class="fill-accent" />
        <rect x={tower.x + 6 + (tower.w - 12) * tower.bar.done} y={look.ground + 7}
          width={(tower.w - 12) * tower.bar.blocked} height="3" rx="1.5" class="fill-alert" />
      {/if}
      <text x={tower.x + tower.w / 2} y={look.ground + PLINTH + 22} text-anchor="middle"
        class={[
          "font-mono transition-colors ease-leave group-hover:fill-text group-hover:ease-arrive",
          "group-focus-visible:fill-text group-focus-visible:ease-arrive",
          tower.picked ? "fill-text" : "fill-text-quiet",
        ]}
        font-size={tower.name.size}>
        <title>{tower.name.whole}</title>
        {tower.name.shown}
      </text>
      {#if tower.line !== null}
        <text x={tower.x + tower.w / 2} y={look.ground + PLINTH + 38} text-anchor="middle"
          class="fill-text-disabled font-mono" font-size={tower.line.size}>
          <title>{tower.line.whole}</title>
          {tower.line.shown}
        </text>
      {/if}
      {#each tower.marks as placed, index (index)}
        <Mark {placed} />
      {/each}
    </g>
  {/each}
</svg>
