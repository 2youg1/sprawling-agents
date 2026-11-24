// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createSignal, For, Show } from "solid-js";
import { say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import { PHASE_FILL } from "../runs/phase";
import { phaseNamed, worn, type Mark, type Stretch, type WearLook } from "./wear";
import "./marks.css";

export interface WearProps {
  readonly scroller: HTMLElement;
  readonly column: HTMLElement;
}

const pct = (share: number): string => `${String(share * 100)}%`;
const TICK = 0.012;

export default function Wear(props: WearProps) {
  const lang = storeValue(ui().lang);
  const [marks, setMarks] = createSignal<readonly Mark[]>([]);
  const [read, setRead] = createSignal<readonly Stretch[]>([]);
  const [thumb, setThumb] = createSignal<Stretch>([0, 1]);
  const [overflows, setOverflows] = createSignal(false);
  let seen: readonly Stretch[] = [];
  let frame = 0;

  function measure(): void {
    frame = 0;
    const { scroller, column } = props;
    const whole = scroller.scrollHeight;
    const overflowing = whole > scroller.clientHeight;
    setOverflows(overflowing);
    if (!overflowing) return;
    const origin = scroller.getBoundingClientRect().top - scroller.scrollTop;
    setMarks(
      [...column.querySelectorAll<HTMLElement>("[data-wear]")].flatMap((block) => {
        const phase = phaseNamed(block.dataset.wear);
        if (phase === undefined) return [];
        const box = block.getBoundingClientRect();
        return [{ top: (box.top - origin) / whole, height: box.height / whole, phase }];
      }),
    );
    seen = worn(seen, scroller.scrollTop, scroller.scrollTop + scroller.clientHeight);
    setRead(seen.map(([a, b]) => [a / whole, (b - a) / whole] as const));
    setThumb([scroller.scrollTop / whole, scroller.clientHeight / whole]);
  }
  function soon(): void {
    if (frame === 0) frame = requestAnimationFrame(measure);
  }
  function moveTo(event: PointerEvent & { readonly currentTarget: HTMLElement }): void {
    const track = event.currentTarget.getBoundingClientRect();
    const share = (event.clientY - track.top) / track.height;
    const { scroller } = props;
    scroller.scrollTop = share * scroller.scrollHeight - scroller.clientHeight / 2;
  }

  createEffect(
    () => ({ ...props }),
    ({ scroller, column }) => {
      const watcher = new ResizeObserver(soon);
      watcher.observe(column);
      watcher.observe(scroller);
      scroller.addEventListener("scroll", soon, { passive: true });
      return () => {
        watcher.disconnect();
        scroller.removeEventListener("scroll", soon);
        if (frame !== 0) cancelAnimationFrame(frame);
        frame = 0;
      };
    },
  );

  return (
    <Show when={overflows()}>
      <div class="absolute inset-y-0 right-0 w-snug narrow:hidden" aria-hidden="true">
        <WearTrack
          hint={say(lang(), "talk_wear_hint")}
          read={read()}
          marks={marks()}
          thumb={thumb()}
          track={{
            onpointerdown: (event) => {
              event.currentTarget.setPointerCapture(event.pointerId);
              moveTo(event);
            },
            onpointermove: (event) => {
              if (event.currentTarget.hasPointerCapture(event.pointerId)) moveTo(event);
            },
          }}
        />
      </div>
    </Show>
  );
}

export function WearTrack(props: WearLook) {
  return (
    <div
      class="wear-track relative h-full cursor-pointer touch-none overflow-hidden"
      title={props.hint}
      onPointerDown={(event) => props.track.onpointerdown(event)}
      onPointerMove={(event) => props.track.onpointermove(event)}
    >
      <div class="wear-line bg-edge" />
      <For each={props.read}>
        {([top, height]) => (
          <div class="wear-worn bg-edge-input" style={{ top: pct(top), height: pct(height) }} />
        )}
      </For>
      <For each={props.marks}>
        {(mark) => (
          <div
            class={["wear-tick", PHASE_FILL[mark.phase]]}
            style={{
              top: pct(Math.max(0, mark.top + mark.height / 2 - TICK / 2)),
              height: pct(TICK),
            }}
          />
        )}
      </For>
      <div
        class="wear-thumb bg-text-faint"
        style={{ top: pct(props.thumb[0]), height: pct(props.thumb[1]) }}
      />
    </div>
  );
}
