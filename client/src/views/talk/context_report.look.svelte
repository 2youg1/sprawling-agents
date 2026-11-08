<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // How the context report is drawn (client D95): one composition ring
  // the person picks a bucket on, the chosen bucket's rows under it, and
  // one foot link to the prompt lens. Every word arrives on `ReportLook`.
  // Empty and Off rows stay out; the close control is the inspector
  // tab's, not a second button here.
  import type { BucketId, ReportLook } from "./context_report";
  import { arcsOf } from "./context_report";

  interface Props {
    readonly look: ReportLook;
  }

  const { look }: Props = $props();

  const arcs = $derived(arcsOf(look.buckets));

  // `undefined` follows the largest bucket; `null` is an explicit clear
  // (centre shows window fullness); a BucketId is a deliberate pick.
  let held = $state<BucketId | null | undefined>(undefined);

  $effect(() => {
    if (held === undefined || held === null) return;
    if (!look.buckets.some((bucket) => bucket.id === held)) held = undefined;
  });

  const picked = $derived.by(() => {
    if (held === null) return null;
    const id = held ?? look.buckets[0]?.id ?? null;
    return look.buckets.find((bucket) => bucket.id === id) ?? null;
  });

  const active = $derived(picked?.id ?? null);

  // Compare against the effective selection (`active`), not only `held`:
  // first paint leaves `held` undefined while the largest bucket still
  // shows as pressed, and that first press must clear rather than stick.
  function pick(id: BucketId): void {
    held = active === id ? null : id;
  }

  function ink(tone: number, on: boolean): string {
    const fade = Math.min(0.85, 0.35 + tone * 0.12);
    const mix = Math.round((1 - fade) * 100);
    const base = `color-mix(in oklch, var(--color-accent) ${String(mix)}%, var(--color-text-faint))`;
    if (on || active === null) return base;
    return `color-mix(in oklch, ${base} 35%, var(--color-track))`;
  }

  // Hit a point on the ring: map pointer angle (from noon, clockwise) to
  // a bucket arc. The SVG is rotated -90deg so 0° is at twelve o'clock;
  // the hit target is a non-focusable layer so the keyboard path stays
  // on the legend chips and a pointer press cannot park focus here.
  function hit(event: MouseEvent & { currentTarget: HTMLElement }): void {
    const box = event.currentTarget.getBoundingClientRect();
    const x = event.clientX - box.left - box.width / 2;
    const y = event.clientY - box.top - box.height / 2;
    const radius = Math.hypot(x, y) / (box.width / 2);
    // Only the thick composition ring is a target, not the hollow centre
    // and not the thin fullness hairline outside it.
    if (radius < 0.55 || radius > 0.92) return;
    let angle = (Math.atan2(y, x) * 180) / Math.PI + 90;
    if (angle < 0) angle += 360;
    const at = (angle / 360) * 100;
    let walk = 0;
    for (const arc of arcs) {
      walk += arc.share;
      if (at <= walk) {
        pick(arc.id);
        return;
      }
    }
  }
</script>

<article class="grid h-full min-h-0 grid-rows-[auto_auto_auto_auto] content-start gap-wide overflow-auto px-wide py-wide">
  <header class="grid gap-snug">
    <p class="text-caption text-text-faint">{look.title}</p>
    <p class="text-figure font-figure figure text-text">
      {look.usedSaid}{#if look.shareSaid}
        <span class="text-text-quiet"> · {look.shareSaid}</span>
      {/if}
    </p>
    <p class="text-note text-text">{look.insight}</p>
    {#if look.warning !== null}
      <p class="text-note text-alert" role="status">{look.warning}</p>
    {/if}
  </header>

  {#if look.buckets.length > 0}
    <div class="grid justify-items-center gap-base">
      <div class="relative size-[12rem]">
        <!-- Pointer-only hit target: not a control, so a press cannot
        park focus here; keyboard stays on the legend chips. -->
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="absolute inset-0 z-1 cursor-pointer rounded-pill" onclick={hit}></div>
        <svg class="pointer-events-none size-full -rotate-90" viewBox="0 0 40 40" aria-hidden="true">
          <circle
            class="pointer-events-none fill-none stroke-track"
            cx="20"
            cy="20"
            r="15.5"
            stroke-width="3.5"
            pathLength="100"
          />
          {#each arcs as arc (arc.id)}
            <circle
              class="pointer-events-none fill-none transition-[stroke,stroke-width,stroke-dasharray,stroke-dashoffset] duration-page ease-arrive"
              cx="20"
              cy="20"
              r="15.5"
              stroke-width={active === arc.id ? 4.2 : 3.5}
              pathLength="100"
              stroke={ink(arc.tone, active === arc.id)}
              stroke-dasharray="{String(arc.share)} {String(100 - arc.share)}"
              stroke-dashoffset={String(-arc.offset)}
            />
          {/each}
          {#if look.fullness !== null}
            <circle
              class="pointer-events-none fill-none stroke-text transition-[stroke-dasharray,stroke-dashoffset] duration-page ease-arrive"
              cx="20"
              cy="20"
              r="18"
              stroke-width="1"
              pathLength="100"
              stroke-dasharray="{String(100 - look.fullPercent)} 100"
              stroke-dashoffset={String(-look.fullPercent)}
            />
          {/if}
        </svg>
        <div class="pointer-events-none absolute inset-0 z-2 grid place-items-center">
          <p class="text-center">
            {#if picked !== null}
              <span class="block text-title font-figure figure text-text">{picked.share}%</span>
              <span class="block max-w-[8rem] truncate text-caption text-text-faint">{picked.said}</span>
            {:else}
              <span class="block text-title font-figure figure text-text">{look.fullPercent}%</span>
              <span class="block text-caption text-text-faint">{look.fullLabel}</span>
            {/if}
          </p>
        </div>
      </div>

      <ul class="flex max-w-[22rem] flex-wrap justify-center gap-tight text-caption" role="list">
        {#each look.buckets as bucket, tone (bucket.id)}
          <li>
            <button
              type="button"
              class="inline-flex items-center gap-snug rounded-control px-snug py-tight wash"
              class:wash-strong={active === bucket.id}
              aria-pressed={active === bucket.id}
              onclick={() => {
                pick(bucket.id);
              }}
            >
              <span
                class="size-[0.55rem] shrink-0 rounded-pill"
                style:background={ink(tone, active === bucket.id)}
                aria-hidden="true"
              ></span>
              <span class="text-text">{bucket.said}</span>
              <span class="figure text-text-quiet tabular-nums">{bucket.share}%</span>
            </button>
          </li>
        {/each}
      </ul>
    </div>

    {#if picked !== null}
      <section class="grid content-start gap-snug border-t border-edge-quiet pt-wide">
        <header class="grid grid-cols-[minmax(0,1fr)_auto] items-baseline gap-snug">
          <h2 class="truncate text-note text-text">{picked.said}</h2>
          <p class="figure text-note text-text-quiet tabular-nums">
            {picked.bytesSaid} · {picked.share}%
          </p>
        </header>
        <div class="summary h-hair overflow-hidden rounded-pill bg-track" aria-hidden="true">
          <div class="h-full bg-accent" style:width="{String(picked.share)}%"></div>
        </div>
        {#if picked.children.length > 0}
          <ul class="grid gap-tight text-caption text-text-quiet" role="list">
            {#each picked.children as child (child.key)}
              <li class="grid grid-cols-[minmax(0,1fr)_auto] gap-snug">
                <span class="truncate font-mono">{child.name}</span>
                <span class="figure tabular-nums">{child.bytesSaid}</span>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="text-caption text-text-faint">{picked.emptyNote}</p>
        {/if}
      </section>
    {:else}
      <p class="text-caption text-text-faint">{look.pickHint}</p>
    {/if}
  {:else}
    <p class="text-note text-text-faint">{look.waiting}</p>
  {/if}

  <footer>
    <a class="text-note text-accent underline-offset-2 hover:underline" href={look.promptHref}>
      {look.openPrompt}
    </a>
  </footer>
</article>
