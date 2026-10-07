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

A pointer resting on it is told what the three layers are, because a
column of coloured dashes says nothing by itself. The track clips what
it draws: a round's tick has a floor of three pixels, and one at the
very foot would otherwise reach past the column and give the page
below it room to scroll.

It is the pointer's scrollbar: pressing the track moves the view there
and dragging follows the pointer. The column itself still scrolls by
wheel, touch and keys, and the native scrollbar's job for a screen
reader is the scroller's, so the bar is hidden from the accessibility
tree. Measuring waits for the next frame, so a burst of deltas costs one
read of the page, not one per delta.

This seat measures and moves the view; what it keeps is `wear.ts`'s and
how the bar is drawn is `wear.look.svelte`'s. -->
<script lang="ts">
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import Look from "./wear.look.svelte";
  import { phaseNamed, worn } from "./wear";
  import type { Mark, Stretch, TrackWire } from "./wear";

  interface Props {
    readonly scroller: HTMLElement;
    readonly column: HTMLElement;
  }

  const { scroller, column }: Props = $props();
  const { lang } = ui();

  // Shares of the whole content, so the bar is drawn in percentages and
  // needs no height of its own.
  let marks = $state<readonly Mark[]>([]);
  let read = $state<readonly Stretch[]>([]);
  let thumb = $state<Stretch>([0, 1]);
  // Whether there is anywhere to scroll to: a column that fits draws no
  // bar, the way a native scrollbar draws none.
  let overflows = $state(false);

  // What has been read, in content pixels.
  let seen: readonly Stretch[] = [];
  let frame = 0;

  function measure(): void {
    frame = 0;
    const whole = scroller.scrollHeight;
    overflows = whole > scroller.clientHeight;
    if (!overflows) return;
    const origin = scroller.getBoundingClientRect().top - scroller.scrollTop;
    marks = [...column.querySelectorAll<HTMLElement>("[data-wear]")].flatMap((block) => {
      const phase = phaseNamed(block.dataset.wear);
      if (phase === undefined) return [];
      const box = block.getBoundingClientRect();
      return [{ top: (box.top - origin) / whole, height: box.height / whole, phase }];
    });
    seen = worn(seen, scroller.scrollTop, scroller.scrollTop + scroller.clientHeight);
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
  function moveTo(event: PointerEvent & { currentTarget: EventTarget & HTMLElement }): void {
    const box = event.currentTarget.getBoundingClientRect();
    const share = (event.clientY - box.top) / box.height;
    scroller.scrollTop = share * scroller.scrollHeight - scroller.clientHeight / 2;
  }

  const track: TrackWire = {
    onpointerdown: (event) => {
      event.currentTarget.setPointerCapture(event.pointerId);
      moveTo(event);
    },
    onpointermove: (event) => {
      if (event.currentTarget.hasPointerCapture(event.pointerId)) moveTo(event);
    },
  };
</script>

<!-- The bar's place is the column's right edge, and a narrow window
leaves the system's scrollbar instead: placement is the seat's, the
drawing inside it the look's. -->
{#if overflows}
  <div class="absolute inset-y-0 right-0 w-snug narrow:hidden" aria-hidden="true">
    <Look hint={say($lang, "talk_wear_hint")} {track} {read} {thumb} {marks} />
  </div>
{/if}
