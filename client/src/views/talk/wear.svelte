<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- The read-wear bar: the conversation's scrollbar, drawn as a map of
what is in it and of what the person has already read (refrain S7.3,
Hill et al. 1992's read wear).

Three layers on one narrow track at the column's right edge:

- **where each round is**, a tick in its phase's colour from
  `runs/phase.ts` - a round that waited on the person, one that worked
  through tools, one that only spoke, and the line a run ended on - read
  off the `data-wear` attribute each block carries, so the bar and the
  thread cannot disagree;
- **what has been read**, every stretch the viewport has shown since the
  room opened, worn a shade darker;
- **where the view is now**, the thumb.

It is the pointer's scrollbar: pressing the track moves the view there
and dragging follows the pointer. The column itself still scrolls by
wheel, touch and keys, and the native scrollbar's job for a screen
reader is the scroller's, so the bar is hidden from the accessibility
tree. Measuring waits for the next frame, so a burst of deltas costs one
read of the page, not one per delta. -->
<script lang="ts">
  import type { Phase } from "../runs/lineage";
  import { PHASES, PHASE_FILL } from "../runs/phase";

  interface Props {
    readonly scroller: HTMLElement;
    readonly column: HTMLElement;
  }

  const { scroller, column }: Props = $props();

  interface Mark {
    readonly top: number;
    readonly height: number;
    readonly phase: Phase;
  }

  // Fractions of the whole content, so the bar is drawn in percentages
  // and needs no height of its own.
  let marks = $state<readonly Mark[]>([]);
  let read = $state<readonly (readonly [number, number])[]>([]);
  let thumb = $state<readonly [number, number]>([0, 1]);
  // Whether there is anywhere to scroll to: a column that fits draws no
  // bar, the way a native scrollbar draws none.
  let overflows = $state(false);

  // What has been read, in content pixels: a short list of disjoint
  // stretches, merged as the view moves.
  let seen: (readonly [number, number])[] = [];
  let frame = 0;

  function isPhase(word: string | undefined): word is Phase {
    return PHASES.some((each) => each === word);
  }

  function wear(from: number, to: number): void {
    const merged: (readonly [number, number])[] = [];
    let start = from;
    let end = to;
    for (const [a, b] of seen) {
      if (b < start || a > end) merged.push([a, b]);
      else {
        start = Math.min(start, a);
        end = Math.max(end, b);
      }
    }
    merged.push([start, end]);
    seen = merged.sort((x, y) => x[0] - y[0]);
  }

  function measure(): void {
    frame = 0;
    const whole = scroller.scrollHeight;
    overflows = whole > scroller.clientHeight;
    if (!overflows) return;
    const origin = scroller.getBoundingClientRect().top - scroller.scrollTop;
    marks = [...column.querySelectorAll<HTMLElement>("[data-wear]")].flatMap((block) => {
      const phase = block.dataset.wear;
      if (!isPhase(phase)) return [];
      const box = block.getBoundingClientRect();
      return [{ top: (box.top - origin) / whole, height: box.height / whole, phase }];
    });
    wear(scroller.scrollTop, scroller.scrollTop + scroller.clientHeight);
    read = seen.map(([a, b]) => [a / whole, (b - a) / whole] as const);
    thumb = [scroller.scrollTop / whole, scroller.clientHeight / whole];
  }

  function soon(): void {
    if (frame === 0) frame = requestAnimationFrame(measure);
  }

  $effect(() => {
    const watcher = new ResizeObserver(soon);
    watcher.observe(column);
    watcher.observe(scroller);
    scroller.addEventListener("scroll", soon, { passive: true });
    return () => {
      watcher.disconnect();
      scroller.removeEventListener("scroll", soon);
      if (frame !== 0) cancelAnimationFrame(frame);
    };
  });

  // The view centred on where the pointer is on the track.
  function moveTo(event: PointerEvent): void {
    const track = event.currentTarget;
    if (!(track instanceof HTMLElement)) return;
    const box = track.getBoundingClientRect();
    const share = (event.clientY - box.top) / box.height;
    scroller.scrollTop = share * scroller.scrollHeight - scroller.clientHeight / 2;
  }

  const pct = (share: number): string => `${String(share * 100)}%`;
</script>

{#if overflows}
  <div
    class="absolute inset-y-0 right-0 w-snug cursor-pointer touch-none narrow:hidden"
    aria-hidden="true"
    onpointerdown={(event) => {
      event.currentTarget.setPointerCapture(event.pointerId);
      moveTo(event);
    }}
    onpointermove={(event) => {
      if (event.currentTarget.hasPointerCapture(event.pointerId)) moveTo(event);
    }}
  >
    <div class="absolute inset-y-0 right-tight w-px bg-edge"></div>
    {#each read as [top, height], at (at)}
      <div class="absolute right-tight w-px bg-edge-input" style:top={pct(top)} style:height={pct(height)}></div>
    {/each}
    <div
      class="absolute right-[3px] w-[3px] rounded-pill bg-raised-hover"
      style:top={pct(thumb[0])}
      style:height={pct(thumb[1])}
    ></div>
    {#each marks as mark, at (at)}
      <div
        class={["absolute right-[3px] min-h-[3px] w-[3px] rounded-pill", PHASE_FILL[mark.phase]]}
        style:top={pct(mark.top)}
        style:height={pct(Math.min(mark.height, 0.01))}
      ></div>
    {/each}
  </div>
{/if}
