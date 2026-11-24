// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For, Match, Show, Switch, createMemo } from "solid-js";
import Glyph from "../parts/glyph.solid";
import type { BoardLook, RowBody, RowLook } from "./board";
import { PHASE_FILL, PHASE_MARK } from "./phase";
import "./board.css";

const TITLE = "text-title font-title";
const INK = { quiet: "text-text-quiet", live: "text-accent", alert: "text-alert" } as const;

function variant<K extends RowBody["kind"]>(
  body: RowBody,
  kind: K,
): body is Extract<RowBody, { kind: K }> {
  return body.kind === kind;
}

function BoardBody(props: { readonly body: RowBody }) {
  function read<K extends RowBody["kind"]>(kind: K) {
    const body = props.body;
    return variant(body, kind) ? body : undefined;
  }
  const building = createMemo(() => read("building"));
  const room = createMemo(() => read("room"));
  const run = createMemo(() => read("run"));
  return (
    <Switch>
      <Match when={building()}>
        {(item) => (
          <>
            <span class="min-w-0 flex-1 truncate font-mono font-label text-text">
              {item().name}
            </span>
            <span
              class={[
                "inline-flex shrink-0 items-center gap-tight font-mono figure",
                item().asking === 0 ? "text-text-quiet" : "text-alert",
              ]}
            >
              <Glyph name="hand" size="sm" />
              {String(item().asking)}
            </span>
            <span class="shrink-0 font-mono figure text-text-quiet">{String(item().runs)}</span>
          </>
        )}
      </Match>
      <Match when={room()}>
        {(item) => (
          <span class="min-w-0 flex-1 truncate font-mono text-text-quiet">{item().name}</span>
        )}
      </Match>
      <Match when={run()}>
        {(item) => {
          const mark = () => PHASE_MARK[item().phase];
          const left = () => `${item().bar.left.toFixed(3)}%`;
          const width = () => `${item().bar.width.toFixed(3)}%`;
          return (
            <>
              <Glyph name={mark().glyph} size="sm" class={["shrink-0", INK[mark().weight]]} />
              <span class="hidden shrink-0 font-mono text-text-quiet @lg/page:inline">
                {item().id}
              </span>
              <span class="min-w-0 flex-1 truncate text-text">{item().title}</span>
              <span
                class={[
                  "hidden max-w-[30%] shrink-0 truncate text-note @lg/page:inline",
                  INK[mark().weight],
                ]}
              >
                {item().said}
              </span>
              <span class="relative h-snug w-[40%] shrink-0" aria-hidden="true">
                <span
                  class="absolute inset-y-0 rounded-pill bg-edge"
                  style={{ left: left(), width: width() }}
                />
                <span
                  class={["absolute inset-y-0 w-snug rounded-pill", PHASE_FILL[item().phase]]}
                  style={{
                    left: `clamp(0px, calc(${left()} + ${width()} - var(--spacing-snug)), calc(100% - var(--spacing-snug)))`,
                  }}
                />
              </span>
            </>
          );
        }}
      </Match>
    </Switch>
  );
}

function BoardRow(props: { readonly row: RowLook }) {
  const expanded = () => props.row.wire["aria-expanded"];
  return (
    <li
      id={props.row.wire.id}
      role={props.row.wire.role}
      data-control="surface"
      aria-level={props.row.wire["aria-level"]}
      aria-expanded={expanded() === undefined ? undefined : expanded() ? "true" : "false"}
      aria-selected={props.row.current ? "true" : "false"}
      onClick={() => props.row.wire.onclick()}
      class={[
        "flex h-step min-w-0 cursor-pointer items-center gap-snug pr-snug text-note transition-colors ease-leave hover:wash hover:ease-arrive",
        props.row.current && "wash-strong shadow-[inset_2px_0_0_var(--color-accent)]",
      ]}
    >
      <span class="shrink-0 pl-snug font-mono whitespace-pre text-text-quiet" aria-hidden="true">
        {props.row.guide}
      </span>
      <BoardBody body={props.row.body} />
    </li>
  );
}

export default function BoardLookView(props: {
  readonly look: BoardLook;
  readonly hold: (tree: HTMLUListElement) => void;
}) {
  const look = () => props.look;
  return (
    <div class="flex min-w-0 flex-col gap-base">
      <header class="flex flex-wrap items-baseline gap-x-base gap-y-snug">
        <Show
          when={look().level === 1}
          fallback={
            <h2 class={TITLE} tabindex={-1}>
              {look().title}
            </h2>
          }
        >
          <h1 class={TITLE} tabindex={-1}>
            {look().title}
          </h1>
        </Show>
        <p class="figure text-note text-text-quiet">{look().counts}</p>
      </header>
      <Show when={look().asking}>
        {(asking) => (
          <nav class="flex flex-col gap-tight text-note" aria-label={asking().label}>
            <span class="text-alert">{asking().label}</span>
            <div class="flex flex-wrap items-center gap-snug">
              <For each={asking().runs} keyed={(run) => run.key}>
                {(run) => (
                  <button
                    type={run().wire.type}
                    data-control="surface"
                    onClick={() => run().wire.onclick()}
                    class="inline-flex items-center gap-tight rounded-pill border border-edge-input px-snug font-mono text-text transition-colors ease-leave hover:bg-raised hover:ease-arrive"
                  >
                    <Glyph name="hand" size="sm" class="text-alert" />
                    {run().text}
                  </button>
                )}
              </For>
            </div>
          </nav>
        )}
      </Show>
      <ul
        class="flex flex-wrap gap-x-wide gap-y-tight text-note text-text-quiet"
        aria-label={look().legend.label}
      >
        <For each={look().legend.phases} keyed={(phase) => phase.phase}>
          {(phase) => (
            <li class="flex items-center gap-snug">
              <span
                class={["inline-block h-snug w-base rounded-pill", PHASE_FILL[phase().phase]]}
                aria-hidden="true"
              />
              {phase().word}
            </li>
          )}
        </For>
      </ul>
      <div
        class="flex min-w-0 items-end gap-snug border-b border-edge pr-snug pb-tight font-mono figure text-note text-text-quiet"
        aria-hidden="true"
      >
        <span class="min-w-0 flex-1" />
        <span class="relative h-base w-[40%] shrink-0">
          <For each={look().folds} keyed={(fold) => fold.minutes}>
            {(fold) => (
              <span
                class={[
                  "absolute top-0 border-l border-edge-input pl-tight",
                  fold().wide && "hidden @lg/page:inline",
                ]}
                style={{ left: `${String(fold().at)}%` }}
              >
                {fold().label}
              </span>
            )}
          </For>
          <span class="absolute top-0 right-0 font-sans text-text">{look().now}</span>
        </span>
      </div>
      <ul
        role={look().tree.role}
        tabindex={look().tree.tabindex}
        aria-label={look().tree["aria-label"]}
        aria-activedescendant={look().tree["aria-activedescendant"]}
        onKeyDown={(event) => look().tree.onkeydown(event)}
        onScroll={(event) => look().tree.onscroll(event)}
        class="runs-board-viewport max-h-[70dvh] min-w-0 overflow-y-auto border-b border-edge"
        style={{
          "--runs-before": `${String(look().pad.above)}px`,
          "--runs-after": `${String(look().pad.below)}px`,
        }}
        ref={props.hold}
      >
        <For each={look().rows} keyed={(row) => row.key}>
          {(row) => <BoardRow row={row()} />}
        </For>
      </ul>
      <p class="text-note text-text-quiet">{look().keys}</p>
    </div>
  );
}
