<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

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
  let marks = $state<readonly Mark[]>([]);
  let read = $state<readonly Stretch[]>([]);
  let thumb = $state<Stretch>([0, 1]);
  let overflows = $state(false);
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

{#if overflows}
  <div class="absolute inset-y-0 right-0 w-snug narrow:hidden" aria-hidden="true">
    <Look hint={say($lang, "talk_wear_hint")} {track} {read} {thumb} {marks} />
  </div>
{/if}
