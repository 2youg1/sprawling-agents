// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { JSX } from "@solidjs/web";
import { createEffect, createMemo, createSignal, For, Show, type Accessor } from "solid-js";
import {
  lookOf,
  turned,
  UNSORTED,
  type Column as TableColumn,
  type HeldLook,
  type Min,
  type Sorted,
  type TableProps as TableModel,
  type TickWire,
} from "./table";

type Render<T> = (row: T) => JSX.Element;
type ReactiveRender<T> = (row: Accessor<T>) => JSX.Element;
type Empty = () => JSX.Element;
export type Column<T> = Omit<TableColumn<T, Render<T>>, "render"> &
  (
    | { readonly render: Render<T>; readonly renderReactive?: never }
    | { readonly render?: never; readonly renderReactive: ReactiveRender<T> }
  );
export type TableProps<T> = Omit<TableModel<T, Render<T>, Empty>, "columns"> & {
  readonly columns: readonly Column<T>[];
};

function renderer<T>(column: Column<T>): ReactiveRender<T> {
  if (column.renderReactive !== undefined) return column.renderReactive;
  const render = column.render;
  return (row) => <>{render(row())}</>;
}

function breadthOf(min: Min | undefined): string | undefined {
  switch (min) {
    case undefined:
      return undefined;
    case "12ch":
      return "min-w-[12ch]";
    case "24ch":
      return "min-w-[24ch]";
    case "figure":
      return "w-figure";
  }
}

function Tick(props: { readonly wire: TickWire }) {
  const [input, setInput] = createSignal<HTMLInputElement>();
  createEffect(
    () => [input(), props.wire.indeterminate] as const,
    ([node, mixed]) => {
      if (node !== undefined) node.indeterminate = mixed;
    },
  );
  return (
    <input
      type={props.wire.type}
      aria-label={props.wire["aria-label"]}
      checked={props.wire.checked}
      onChange={(event) => props.wire.onchange(event)}
      ref={(node) => setInput(node)}
    />
  );
}

function Drawn<T>(props: { readonly held: HeldLook<T, ReactiveRender<T>> }) {
  const field = () => {
    const held = props.held;
    return held.kind === "field" ? held.field : undefined;
  };
  const drawn = () => {
    const held = props.held;
    return held.kind === "drawn" ? held : undefined;
  };
  return (
    <Show
      when={field()}
      fallback={<Show when={drawn()}>{(held) => held().render(() => held().row)}</Show>}
    >
      {(wire) => (
        <input
          aria-label={wire()["aria-label"]}
          value={wire().value}
          onChange={(event) => wire().onchange(event)}
          class="w-full min-w-0 rounded-control bg-chrome px-snug py-tight font-mono text-note text-text"
        />
      )}
    </Show>
  );
}

export default function Table<T>(props: TableProps<T>) {
  const [sorted, setSorted] = createSignal<Sorted>(UNSORTED);
  const look = createMemo(() =>
    lookOf(
      {
        ...props,
        columns: props.columns.map((column) => ({ ...column, render: renderer(column) })),
      },
      sorted(),
      (key) => setSorted((order) => turned(order, key)),
    ),
  );
  return (
    <div class="max-h-output w-full overflow-auto rounded-card border border-edge-panel">
      <Show when={look().lines.length > 0} fallback={look().empty?.()}>
        <table class="w-full border-collapse text-body">
          <caption class="sr-only">{look().caption}</caption>
          <thead class="sticky top-0 bg-raised">
            <tr class="border-b border-edge">
              <Show when={look().all}>
                {(wire) => (
                  <th class="w-glyph px-base py-tight text-left">
                    <Tick wire={wire()} />
                  </th>
                )}
              </Show>
              <For each={look().heads} keyed={(head) => head.key}>
                {(head) => (
                  <th
                    {...head().cell}
                    class={[
                      "whitespace-nowrap px-base py-tight text-left text-note font-label text-text-quiet",
                      breadthOf(head().min),
                      head().summary && "summary",
                    ]}
                  >
                    <Show when={head().sort} fallback={head().header}>
                      {(sort) => (
                        <button
                          data-control=""
                          type={sort().type}
                          onClick={() => sort().onclick()}
                          class="text-note font-label text-text-quiet transition-colors ease-leave hover:text-text hover:ease-arrive"
                        >
                          {head().header}
                        </button>
                      )}
                    </Show>
                  </th>
                )}
              </For>
            </tr>
          </thead>
          <tbody>
            <For each={look().lines} keyed={(line) => line.key}>
              {(line) => (
                <tr class="h-control-lg border-b border-edge transition-[background-color] ease-leave hover:wash hover:ease-arrive focus-within:wash focus-within:ease-arrive last:border-b-0">
                  <Show when={line().tick}>
                    {(wire) => (
                      <td class="px-base py-tight">
                        <Tick wire={wire()} />
                      </td>
                    )}
                  </Show>
                  <For each={line().cells} keyed={(cell) => cell.key}>
                    {(cell) => (
                      <td
                        class={[
                          "px-base py-tight text-text",
                          breadthOf(cell().min),
                          cell().summary && "summary",
                        ]}
                      >
                        <Show
                          when={cell().min === "figure"}
                          fallback={<Drawn held={cell().held} />}
                        >
                          <div class="figure-box [inline-size:calc(var(--spacing-figure)+var(--spacing-wide))]">
                            <Drawn held={cell().held} />
                          </div>
                        </Show>
                      </td>
                    )}
                  </For>
                </tr>
              )}
            </For>
          </tbody>
        </table>
      </Show>
    </div>
  );
}
