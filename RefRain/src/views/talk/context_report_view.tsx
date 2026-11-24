// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createMemo, createSignal, For, Show } from "solid-js";
import { QUERIES } from "../../core/asking";
import { storeValue } from "../../core/solid_store";
import { readable } from "../../core/store";
import { ui } from "../../ui";
import type { Address, Answer, RunId } from "../../wire";
import { arcsOf, reportOf, type BucketId, type ReportLook } from "./context_report";
import { contextOf, lockOfConfig, remindersOf } from "./gauge";

const UNASKED = readable<Answer | undefined>(undefined);

export default function ContextReport(props: {
  readonly run: RunId;
  readonly room: Address | null;
}) {
  const u = ui();
  const lang = storeValue(u.lang);
  const endpoints = storeValue(u.conn.asking.ask(QUERIES.endpoints));
  const prefix = storeValue(() => u.conn.asking.ask({ prefix: { run: props.run } }));
  const shape = storeValue(() => u.conn.asking.ask({ prompt_shape: { run: props.run } }));
  const rounds = storeValue(() => u.conn.asking.ask({ rounds: { run: props.run } }));
  const config = storeValue(() =>
    props.room === null ? UNASKED : u.conn.asking.ask({ config: { addr: props.room } }),
  );
  const look = createMemo(() => {
    const head = prefix();
    const prompt = shape();
    const history = rounds();
    const model = endpoints();
    const settings = config();
    return reportOf(
      lang(),
      props.run,
      head !== undefined && "prefix" in head ? head.prefix : null,
      prompt !== undefined && "prompt_shape" in prompt ? prompt.prompt_shape : null,
      contextOf(
        history !== undefined && "rounds" in history ? history.rounds.turns : [],
        model !== undefined && "endpoints" in model ? model.endpoints.endpoints : [],
        remindersOf(settings),
        lockOfConfig(settings),
      ),
    );
  });
  return <ContextReportLook look={look()} />;
}

export function ContextReportLook(props: { readonly look: ReportLook }) {
  const arcs = createMemo(() => arcsOf(props.look.buckets));
  const [held, setHeld] = createSignal<BucketId | null | undefined>(undefined);
  const picked = createMemo(() => {
    const current = held();
    if (current === null) return undefined;
    return props.look.buckets.find((bucket) => bucket.id === current) ?? props.look.buckets[0];
  });
  const active = createMemo(() => picked()?.id ?? null);
  createEffect(
    () => {
      const current = held();
      return (
        current !== null &&
        current !== undefined &&
        !props.look.buckets.some((bucket) => bucket.id === current)
      );
    },
    (missing) => {
      if (missing) setHeld(undefined);
    },
  );

  function pick(id: BucketId): void {
    setHeld(active() === id ? null : id);
  }
  function ink(tone: number, on: boolean): string {
    const fade = Math.min(0.85, 0.35 + tone * 0.12);
    const mix = Math.round((1 - fade) * 100);
    const base = `color-mix(in oklch, var(--color-accent) ${String(mix)}%, var(--color-text-faint))`;
    return on || active() === null ? base : `color-mix(in oklch, ${base} 35%, var(--color-track))`;
  }
  function hit(event: MouseEvent & { currentTarget: HTMLDivElement }): void {
    const box = event.currentTarget.getBoundingClientRect();
    const x = event.clientX - box.left - box.width / 2;
    const y = event.clientY - box.top - box.height / 2;
    const radius = Math.hypot(x, y) / (box.width / 2);
    if (radius < 0.55 || radius > 0.92) return;
    const angle = (Math.atan2(y, x) * 180) / Math.PI + 90;
    const at = (((angle + 360) % 360) / 360) * 100;
    const arc = arcs().find((arc) => at >= arc.offset && at <= arc.offset + arc.share);
    if (arc !== undefined) pick(arc.id);
  }

  return (
    <article class="grid h-full min-h-0 grid-rows-[auto_auto_auto_auto] content-start gap-wide overflow-auto px-wide py-wide">
      <header class="grid gap-snug">
        <p class="text-caption text-text-faint">{props.look.title}</p>
        <p class="text-figure font-figure figure text-text">
          {props.look.usedSaid}
          <Show when={props.look.shareSaid}>
            {(share) => <span class="text-text-quiet"> · {share()}</span>}
          </Show>
        </p>
        <p class="text-note text-text">{props.look.insight}</p>
        <Show when={props.look.warning}>
          {(warning) => (
            <p class="text-note text-alert" role="status">
              {warning()}
            </p>
          )}
        </Show>
      </header>
      <Show
        when={props.look.buckets.length > 0}
        fallback={<p class="text-note text-text-faint">{props.look.waiting}</p>}
      >
        <div class="grid justify-items-center gap-base">
          <div class="relative size-[12rem]">
            <div
              aria-hidden="true"
              class="absolute inset-0 z-1 cursor-pointer rounded-pill"
              onClick={hit}
            />
            <svg
              class="pointer-events-none size-full -rotate-90"
              viewBox="0 0 40 40"
              aria-hidden="true"
            >
              <circle
                class="pointer-events-none fill-none stroke-track"
                cx="20"
                cy="20"
                r="15.5"
                stroke-width="3.5"
                pathLength="100"
              />
              <For each={arcs()} keyed={(arc) => arc.id}>
                {(arc) => (
                  <circle
                    class="pointer-events-none fill-none transition-[stroke,stroke-width,stroke-dasharray,stroke-dashoffset] duration-page ease-arrive"
                    cx="20"
                    cy="20"
                    r="15.5"
                    stroke-width={active() === arc().id ? 4.2 : 3.5}
                    pathLength="100"
                    stroke={ink(arc().tone, active() === arc().id)}
                    stroke-dasharray={`${String(arc().share)} ${String(100 - arc().share)}`}
                    stroke-dashoffset={String(-arc().offset)}
                  />
                )}
              </For>
              <Show when={props.look.fullness !== null}>
                <circle
                  class="pointer-events-none fill-none stroke-text transition-[stroke-dasharray,stroke-dashoffset] duration-page ease-arrive"
                  cx="20"
                  cy="20"
                  r="18"
                  stroke-width="1"
                  pathLength="100"
                  stroke-dasharray={`${String(100 - props.look.fullPercent)} 100`}
                  stroke-dashoffset={String(-props.look.fullPercent)}
                />
              </Show>
            </svg>
            <div class="pointer-events-none absolute inset-0 z-2 grid place-items-center">
              <p class="text-center">
                <Show
                  when={picked()}
                  fallback={
                    <>
                      <span class="block text-title font-figure figure text-text">
                        {props.look.fullPercent}%
                      </span>
                      <span class="block text-caption text-text-faint">{props.look.fullLabel}</span>
                    </>
                  }
                >
                  {(bucket) => (
                    <>
                      <span class="block text-title font-figure figure text-text">
                        {bucket().share}%
                      </span>
                      <span class="block max-w-[8rem] truncate text-caption text-text-faint">
                        {bucket().said}
                      </span>
                    </>
                  )}
                </Show>
              </p>
            </div>
          </div>
          <ul
            class="flex max-w-[22rem] flex-wrap justify-center gap-tight text-caption"
            role="list"
          >
            <For each={props.look.buckets} keyed={(bucket) => bucket.id}>
              {(bucket, tone) => (
                <li>
                  <button
                    type="button"
                    data-control=""
                    aria-pressed={active() === bucket().id ? "true" : "false"}
                    class={[
                      "inline-flex items-center gap-snug rounded-control px-snug py-tight wash",
                      active() === bucket().id && "wash-strong",
                    ]}
                    onClick={() => pick(bucket().id)}
                  >
                    <span
                      class="size-[0.55rem] shrink-0 rounded-pill"
                      style={{ background: ink(tone(), active() === bucket().id) }}
                      aria-hidden="true"
                    />
                    <span class="text-text">{bucket().said}</span>
                    <span class="figure text-text-quiet tabular-nums">{bucket().share}%</span>
                  </button>
                </li>
              )}
            </For>
          </ul>
        </div>
        <Show
          when={picked()}
          fallback={<p class="text-caption text-text-faint">{props.look.pickHint}</p>}
        >
          {(bucket) => (
            <section class="grid content-start gap-snug border-t border-edge-quiet pt-wide">
              <header class="grid grid-cols-[minmax(0,1fr)_auto] items-baseline gap-snug">
                <h2 class="truncate text-note text-text">{bucket().said}</h2>
                <p class="figure text-note text-text-quiet tabular-nums">
                  {bucket().bytesSaid} · {bucket().share}%
                </p>
              </header>
              <div class="summary h-hair overflow-hidden rounded-pill bg-track" aria-hidden="true">
                <div class="h-full bg-accent" style={{ width: `${String(bucket().share)}%` }} />
              </div>
              <Show
                when={bucket().children.length > 0}
                fallback={<p class="text-caption text-text-faint">{bucket().emptyNote}</p>}
              >
                <ul class="grid gap-tight text-caption text-text-quiet" role="list">
                  <For each={bucket().children} keyed={(child) => child.key}>
                    {(child) => (
                      <li class="grid grid-cols-[minmax(0,1fr)_auto] gap-snug">
                        <span class="truncate font-mono">{child().name}</span>
                        <span class="figure tabular-nums">{child().bytesSaid}</span>
                      </li>
                    )}
                  </For>
                </ul>
              </Show>
            </section>
          )}
        </Show>
      </Show>
      <footer>
        <a
          class="text-note text-accent underline-offset-2 hover:underline"
          href={props.look.promptHref}
        >
          {props.look.openPrompt}
        </a>
      </footer>
    </article>
  );
}
