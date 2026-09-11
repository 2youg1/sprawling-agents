// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Many rows of the same shape, chosen among and corrected in place: the
// table a provider's two hundred models are admitted from.
//
// A column carries its own three abilities rather than the table
// carrying three parallel lists: how a cell is drawn, whether the column
// can be ordered by, and whether its value can be corrected in place.
// The empty state is the caller's element, because what to do about an
// empty table is knowledge of the page, not of the table.

import { For, Show, createMemo, createSignal, type JSX } from "solid-js";

export interface Column<T> {
  readonly key: string;
  // Already in the person's language.
  readonly header: string;
  readonly render: (row: T) => JSX.Element;
  // Present makes the column sortable.
  readonly compare?: (a: T, b: T) => number;
  // Present makes the cell an input, and the table hands back what was
  // typed rather than deciding what it means.
  readonly editable?: {
    readonly text: (row: T) => string;
    readonly onEdit: (row: T, text: string) => void;
  };
}

export interface Selection<T> {
  readonly picked: (row: T) => boolean;
  readonly onPick: (row: T, on: boolean) => void;
  // The accessible name of the header checkbox.
  readonly allLabel: string;
  readonly onPickAll: (on: boolean) => void;
  readonly allPicked: () => boolean;
}

export interface TableProps<T> {
  // The accessible name of the table.
  readonly caption: string;
  readonly columns: readonly Column<T>[];
  readonly rows: readonly T[];
  // Both the identity of a row and the accessible name of its checkbox.
  readonly keyOf: (row: T) => string;
  readonly selection?: Selection<T>;
  readonly empty?: JSX.Element;
}

type Order = "up" | "down";

export function Table<T>(
  props: TableProps<T>,
) {
  const [by, setBy] = createSignal<string | null>(null);
  const [order, setOrder] = createSignal<Order>("up");

  const ordered = createMemo(() => {
    const key = by();
    if (key === null) return props.rows;
    const column = props.columns.find((each) => each.key === key);
    const compare = column?.compare;
    if (compare === undefined) return props.rows;
    const sorted = [...props.rows].sort(compare);
    return order() === "up" ? sorted : sorted.reverse();
  });
  const turn = (key: string) => {
    if (by() === key) {
      setOrder(order() === "up" ? "down" : "up");
      return;
    }
    setBy(key);
    setOrder("up");
  };
  const said = (key: string): "ascending" | "descending" | "none" => {
    if (by() !== key) return "none";
    return order() === "up" ? "ascending" : "descending";
  };

  return (
    <div class="max-h-output w-full overflow-auto rounded-card border border-g3">
      <Show when={props.rows.length > 0} fallback={props.empty}>
        <table class="w-full border-collapse text-body">
          <caption class="sr-only">{props.caption}</caption>
          <thead class="sticky top-0 z-10 bg-g2">
            <tr>
              <Show when={props.selection}>
                {(selection) => (
                  <th class="w-glyph px-base py-snug text-left">
                    <input
                      type="checkbox"
                      class="accent-accent"
                      aria-label={selection().allLabel}
                      checked={selection().allPicked()}
                      onChange={(event) => {
                        selection().onPickAll(event.currentTarget.checked);
                      }}
                    />
                  </th>
                )}
              </Show>
              <For each={props.columns}>
                {(column) => (
                  <th class="px-base py-snug text-left text-note font-label text-text-quiet" aria-sort={said(column.key)}>
                    <Show when={column.compare !== undefined} fallback={column.header}>
                      <button
                        type="button"
                        class="text-note font-label text-text-quiet hover:text-text"
                        onClick={() => {
                          turn(column.key);
                        }}
                      >
                        {column.header}
                      </button>
                    </Show>
                  </th>
                )}
              </For>
            </tr>
          </thead>
          <tbody>
            <For each={ordered()}>
              {(row) => (
                <tr class="border-t border-g2">
                  <Show when={props.selection}>
                    {(selection) => (
                      <td class="px-base py-snug">
                        <input
                          type="checkbox"
                          class="accent-accent"
                          aria-label={props.keyOf(row)}
                          checked={selection().picked(row)}
                          onChange={(event) => {
                            selection().onPick(row, event.currentTarget.checked);
                          }}
                        />
                      </td>
                    )}
                  </Show>
                  <For each={props.columns}>
                    {(column) => (
                      <td class="px-base py-snug text-text">
                        <Show when={column.editable} fallback={column.render(row)}>
                          {(editable) => (
                            <input
                              class="w-full min-w-0 rounded-control bg-g1 px-snug py-tight font-mono text-note text-text outline-none"
                              aria-label={`${column.header} ${props.keyOf(row)}`}
                              value={editable().text(row)}
                              onChange={(event) => {
                                editable().onEdit(row, event.currentTarget.value);
                              }}
                            />
                          )}
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
