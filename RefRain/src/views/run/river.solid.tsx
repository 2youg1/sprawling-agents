// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For, Show, createEffect, createMemo, createSignal } from "solid-js";
import { POSTURE_WORD } from "../../core/doing";
import { fill, say, type Key } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { hhmmssms, isoInstant, lasted } from "../../core/time";
import { ui } from "../../ui";
import type { Turn } from "../../wire";
import { tookOf, tookWords } from "../talk/timing";
import { SHARES, callsOf, columnsOf, stretchesOf, windowOf, type Share } from "./lanes";

const WORD: Record<Share, Key> = {
  model: POSTURE_WORD.thinking,
  tool: POSTURE_WORD.calling,
  person: POSTURE_WORD.waiting,
};
const FILL: Record<Share, string> = {
  model: "bg-accent",
  tool: "bg-accent-solid",
  person: "bg-alert",
};
const ROW = "--spacing-control-sm";

export default function River(props: {
  readonly turns: readonly Turn[];
  readonly from: number;
  readonly to: number;
  readonly tail: Share | null;
}) {
  const lang = storeValue(ui().lang);
  const [track, setTrack] = createSignal<HTMLElement>();
  const [list, setList] = createSignal<HTMLElement>();
  const [width, setWidth] = createSignal(0);
  const [height, setHeight] = createSignal(0);
  const [top, setTop] = createSignal(0);
  const [row, setRow] = createSignal(0);
  const whole = createMemo(() => Math.max(1, props.to - props.from));
  const stretches = createMemo(() => stretchesOf(props.turns, props.to, props.tail));
  const columns = createMemo(() => Math.max(1, Math.floor(width() / 2)));
  const lanes = createMemo(() => columnsOf(stretches(), props.from, props.to, columns()));
  const calls = createMemo(() => callsOf(props.turns));
  const rows = createMemo(() => windowOf(calls().length, row(), top(), height()));
  function percent(share: Share): string {
    const taken = stretches()
      .filter((item) => item.share === share)
      .reduce((sum, item) => sum + item.to - item.from, 0);
    return `${String(Math.round((taken / whole()) * 100))}%`;
  }
  createEffect(
    () => [track(), list()] as const,
    ([rail, entries]) => {
      let frame = 0;
      const size =
        entries === undefined
          ? 0
          : Number.parseFloat(getComputedStyle(entries).getPropertyValue(ROW));
      setRow(Number.isFinite(size) ? size : 0);
      const measure = () => {
        setWidth(rail?.clientWidth ?? 0);
        setHeight(entries?.clientHeight ?? 0);
      };
      const observer = new ResizeObserver(() => {
        cancelAnimationFrame(frame);
        frame = requestAnimationFrame(measure);
      });
      if (rail !== undefined) observer.observe(rail);
      if (entries !== undefined) observer.observe(entries);
      measure();
      return () => {
        observer.disconnect();
        cancelAnimationFrame(frame);
      };
    },
  );
  return (
    <section class="flex flex-col gap-base pt-base">
      <div role="img" aria-label={say(lang(), "run_river")} class="flex flex-col gap-snug">
        <div class="flex justify-between pl-figure font-mono text-note text-text-quiet">
          <span>{lasted(0)}</span>
          <span>{lasted(whole() / 2)}</span>
          <span>{lasted(whole())}</span>
        </div>
        <For each={SHARES}>
          {(share, index) => (
            <div class="flex items-center gap-snug">
              <div class="flex w-figure shrink-0 flex-col text-note">
                <span class="truncate text-text">{say(lang(), WORD[share])}</span>
                <span class="font-mono text-text-quiet">{percent(share)}</span>
              </div>
              <div
                class="relative h-control-sm flex-1 border-t border-edge"
                ref={(node) => {
                  if (index() === 0) setTrack(node);
                }}
              >
                <For
                  each={lanes().filter((item) => item.share === share)}
                  keyed={(item) => item.first}
                >
                  {(item) => (
                    <span
                      class={["absolute top-1/2 h-dot -translate-y-1/2 rounded-pill", FILL[share]]}
                      style={{
                        left: `${String((item().first / columns()) * 100)}%`,
                        width: `${String(((item().end - item().first) / columns()) * 100)}%`,
                      }}
                    />
                  )}
                </For>
              </div>
            </div>
          )}
        </For>
      </div>
      <h2 class="flex justify-between text-note text-text-faint">
        <span>
          {say(lang(), "run_calls")} <span class="figure text-text-quiet">{calls().length}</span>
        </span>
        <span>{say(lang(), "run_calls_axis")}</span>
      </h2>
      <Show
        when={calls().length > 0}
        fallback={<p class="text-note text-text-faint">{say(lang(), "run_no_calls")}</p>}
      >
        <div
          class="max-h-tree overflow-auto border-y border-edge"
          ref={(node) => {
            setList(node);
          }}
          onScroll={(event) => {
            setTop(event.currentTarget.scrollTop);
          }}
        >
          <div style={{ height: `calc(var(${ROW}) * ${String(rows().first)})` }} />
          <ol class="@container text-note">
            <For each={calls().slice(rows().first, rows().end)}>
              {(placed) => {
                const took = tookOf(placed.call);
                const answered = placed.call.answered ?? null;
                return (
                  <li class="grid h-control-sm grid-cols-[14ch_minmax(0,1fr)_10ch] items-center gap-x-base whitespace-nowrap @min-[40rem]:grid-cols-[14ch_9ch_12ch_minmax(0,1fr)_10ch]">
                    {answered === null ? (
                      <span class="figure text-text-faint">{say(lang(), "run_call_running")}</span>
                    ) : (
                      <time class="figure text-text-faint" datetime={isoInstant(answered)}>
                        {hhmmssms(answered)}
                      </time>
                    )}
                    <span class="hidden truncate text-text-faint @min-[40rem]:block">
                      {fill(say(lang(), "run_turn_n"), { n: String(placed.turn) })}
                    </span>
                    <span
                      class={[
                        "hidden truncate font-mono @min-[40rem]:block",
                        placed.call.outcome === "failed" ? "text-alert" : "text-text",
                      ]}
                    >
                      {placed.call.tool}
                    </span>
                    <span class="min-w-0 truncate font-mono text-text-quiet">
                      {placed.call.subject ?? placed.call.tool}
                    </span>
                    <span class="figure text-right text-text-faint">
                      {took === null ? "" : tookWords(took, lang())}
                    </span>
                  </li>
                );
              }}
            </For>
          </ol>
          <div style={{ height: `calc(var(${ROW}) * ${String(calls().length - rows().end)})` }} />
        </div>
      </Show>
    </section>
  );
}
