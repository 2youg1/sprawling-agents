// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createMemo, createSignal, For, Show } from "solid-js";
import { fill, say } from "../core/lang";
import { FULL, rows, type Plot } from "../core/monitor";
import { storeValue } from "../core/solid_store";
import { ui } from "../ui";
import type { Sample } from "../wire";
import Page from "./parts/page.solid";
import type { Rank } from "./parts/page";
import Segmented from "./parts/segmented.solid";

export const BEATS = ["50", "100", "250", "500", "1000"] as const;
export type BeatChoice = (typeof BEATS)[number];

export function beatOf(latest: Sample | undefined): BeatChoice | null {
  return BEATS.find((beat) => Number(beat) === latest?.beat_ms) ?? null;
}

export interface MonitorProps {
  readonly samples: readonly Sample[];
  readonly watch: () => () => void;
  readonly beat?: ((ms: number) => void) | undefined;
  readonly rank?: Rank | undefined;
}

const POINT_PX = 3;
const PLOT =
  "col-span-full block h-plot w-full min-w-0 pt-tight @min-[40rem]:col-span-1 @min-[40rem]:pt-0";

function barsOf(plot: Plot, points: number): string {
  const first = points - plot.bars.length;
  return plot.bars
    .map((height, at) => `M${first + at} ${FULL - height}h0.7V${FULL}h-0.7Z`)
    .join("");
}

export default function Monitor(props: MonitorProps) {
  const lang = storeValue(ui().lang);
  const [panel, setPanel] = createSignal(0);
  const [measure, setMeasure] = createSignal<HTMLElement>();
  createEffect(
    () => props.watch,
    (watch) => watch(),
  );
  createEffect(measure, (plot) => {
    if (plot === undefined) return;
    const resize = () => setPanel(plot.clientWidth);
    const observed = new ResizeObserver(resize);
    observed.observe(plot);
    resize();
    return () => observed.disconnect();
  });

  const points = () => Math.floor(panel() / POINT_PX);
  const shown = createMemo(() => rows(props.samples, points()));
  const held = () => beatOf(props.samples.at(-1));
  const beats = () =>
    BEATS.map((value) => ({ value, label: fill(say(lang(), "monitor_beat_ms"), { n: value }) }));
  return (
    <Page
      title={say(lang(), "monitor_title")}
      rank={props.rank}
      aside={
        <Show when={props.beat}>
          {(setBeat) => (
            <>
              <span class="text-note text-text-quiet">{say(lang(), "monitor_beat")}</span>
              <Segmented
                label={say(lang(), "monitor_beat")}
                options={beats()}
                held={held()}
                onPick={(value) => setBeat()(Number(value))}
              />
            </>
          )}
        </Show>
      }
    >
      <section
        class="@container relative flex min-w-0 flex-col"
        aria-label={say(lang(), "monitor_title")}
      >
        <Show
          when={shown().length > 0}
          fallback={<p class="text-note text-text-faint">{say(lang(), "monitor_waiting")}</p>}
        >
          <ul>
            <For each={shown()} keyed={(row) => row.label}>
              {(row, index) => {
                const across = () => Math.max(points(), row().plot.bars.length, 1);
                return (
                  <li class="grid grid-cols-[minmax(0,1fr)_14ch] items-center gap-x-gutter border-b border-edge py-snug @min-[40rem]:grid-cols-[minmax(0,28ch)_14ch_minmax(0,1fr)]">
                    <span class="flex min-w-0 flex-col">
                      <span class="truncate text-note text-text-quiet">
                        {say(lang(), row().label)}
                      </span>
                      <span class="figure truncate text-note text-text-faint">
                        {fill(say(lang(), "monitor_band"), { p50: row().p50, p99: row().p99 })}
                      </span>
                    </span>
                    <span class="figure text-right text-text">{row().reading}</span>
                    <span
                      class={PLOT}
                      ref={(node) => {
                        if (index() === 0) setMeasure(node);
                      }}
                    >
                      <svg
                        aria-hidden="true"
                        class="block h-full w-full"
                        viewBox={`0 0 ${across()} ${FULL}`}
                        preserveAspectRatio="none"
                      >
                        <rect
                          x="0"
                          y={FULL - row().plot.p99}
                          width={across()}
                          height={Math.max(row().plot.p99 - row().plot.p50, 1)}
                          class="fill-accent/15"
                        />
                        <path d={barsOf(row().plot, across())} class="fill-accent" />
                      </svg>
                    </span>
                  </li>
                );
              }}
            </For>
          </ul>
        </Show>
      </section>
    </Page>
  );
}
