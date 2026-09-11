// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The one list that opens over something else: one column or three,
// arrow keys to choose, Tab to change column, Enter to apply, Escape to
// close. The combined selector beside the composer and the `/` menu are
// the same list with different rows.
//
// Two triggers want two different things from the keyboard, and that is
// the only fork in here. A button hands the keyboard over: the popover
// takes focus and gives it back when it closes. A text box somebody is
// still typing in cannot, so it takes the handler instead through
// `bind` and forwards the keys it does not want itself. One key table
// either way, which is the point.

import { For, Show, createEffect, createMemo, createSignal, onCleanup, onMount } from "solid-js";

export interface PopoverItem {
  readonly id: string;
  readonly label: string;
  // The right-hand column of a row: a grammar, an endpoint, an address.
  readonly hint?: string;
  // What is applied now, marked rather than merely pre-selected: a
  // cursor says where a person is, not what the city is doing.
  readonly chosen?: boolean;
}

export interface PopoverColumn {
  readonly id: string;
  readonly label: string;
  readonly items: readonly PopoverItem[];
}

export interface PopoverProps {
  readonly label: string;
  readonly columns: readonly PopoverColumn[];
  readonly onApply: (column: PopoverColumn, item: PopoverItem) => void;
  readonly onClose: () => void;
  readonly bind?: ((keys: (event: KeyboardEvent) => boolean) => void) | undefined;
}

// Unique within the document, so `aria-activedescendant` points at one
// row and not at every popover that ever opened.
let minted = 0;

function focusable(held: EventTarget | null): HTMLElement | null {
  return held instanceof HTMLElement ? held : null;
}

export function Popover(props: PopoverProps) {
  minted += 1;
  const seat = `popover-${String(minted)}`;
  const [column, setColumn] = createSignal(0);
  const [cursor, setCursor] = createSignal(0);
  const [lists, setLists] = createSignal<readonly HTMLElement[]>([]);

  const here = createMemo(() => props.columns.at(column()));
  // A list that shrank under a cursor - somebody typed another letter -
  // leaves the cursor on a row that is no longer there.
  createEffect(() => {
    const rows = here()?.items.length ?? 0;
    if (cursor() >= rows) {
      setCursor(rows === 0 ? 0 : rows - 1);
    }
  });
  const rowSeat = (at: number, row: number) => `${seat}-${String(at)}-${String(row)}`;

  const move = (by: number) => {
    const rows = here()?.items.length ?? 0;
    if (rows === 0) return;
    setCursor((at) => Math.min(rows - 1, Math.max(0, at + by)));
  };
  const step = (by: number) => {
    const count = props.columns.length;
    if (count < 2) return;
    setColumn((at) => (at + by + count) % count);
    setCursor(0);
  };
  const apply = () => {
    const column = here();
    const item = column?.items.at(cursor());
    if (column === undefined || item === undefined) return;
    props.onApply(column, item);
  };

  // Answers whether the popover used the key, so a text box that
  // forwards its keys knows whether to let the character through.
  const keys = (event: KeyboardEvent): boolean => {
    switch (event.key) {
      case "ArrowDown":
        move(1);
        return true;
      case "ArrowUp":
        move(-1);
        return true;
      case "Home":
        setCursor(0);
        return true;
      case "End":
        setCursor(Math.max(0, (here()?.items.length ?? 0) - 1));
        return true;
      case "Tab":
        step(event.shiftKey ? -1 : 1);
        return true;
      case "Enter":
        apply();
        return true;
      case "Escape":
        props.onClose();
        return true;
      default:
        return false;
    }
  };

  onMount(() => {
    if (props.bind !== undefined) {
      props.bind(keys);
      return;
    }
    // The trigger is what had the keyboard a moment ago, and it is what
    // gets it back: a person who opened a menu and closed it is where
    // they were, not at the top of the page.
    const trigger = focusable(document.activeElement);
    onCleanup(() => {
      trigger?.focus();
    });
  });
  // The column that holds the cursor is the one that holds the focus.
  createEffect(() => {
    if (props.bind !== undefined) return;
    lists().at(column())?.focus();
  });

  return (
    <div
      class="absolute bottom-full left-0 z-10 mb-snug w-full rounded-panel border border-g3 bg-g1 p-snug shadow-composer"
      role="dialog"
      aria-label={props.label}
    >
      <div class="flex gap-snug">
        <For each={props.columns}>
          {(each, at) => (
            <div class="flex min-w-0 flex-1 flex-col">
              <div class="mb-tight px-snug text-note text-text-disabled">{each.label}</div>
              <ul
                ref={(node) => {
                  setLists((held) => {
                    const next = [...held];
                    next[at()] = node;
                    return next;
                  });
                }}
                class="max-h-palette overflow-y-auto outline-none"
                role="listbox"
                aria-label={each.label}
                tabindex={props.bind === undefined && at() === column() ? 0 : -1}
                aria-activedescendant={at() === column() ? rowSeat(at(), cursor()) : undefined}
                onKeyDown={(event) => {
                  if (props.bind !== undefined) return;
                  if (keys(event)) {
                    event.preventDefault();
                  }
                }}
              >
                <For each={each.items}>
                  {(item, row) => (
                    <li
                      id={rowSeat(at(), row())}
                      role="option"
                      aria-selected={at() === column() && row() === cursor()}
                      class={`flex cursor-pointer items-center justify-between gap-snug rounded-control px-snug py-tight text-body ${
                        at() === column() && row() === cursor() ? "bg-g2" : ""
                      } ${item.chosen === true ? "text-text" : "text-text-quiet"}`}
                      onMouseEnter={() => {
                        setColumn(at());
                        setCursor(row());
                      }}
                      onClick={() => {
                        props.onApply(each, item);
                      }}
                    >
                      <span class="truncate">
                        <Show when={item.chosen === true}>
                          <span class="mr-tight inline-block size-dot rounded-pill bg-accent align-middle" />
                        </Show>
                        {item.label}
                      </span>
                      <Show when={item.hint}>
                        {(hint) => (
                          <span class="shrink-0 font-mono text-note text-text-disabled">{hint()}</span>
                        )}
                      </Show>
                    </li>
                  )}
                </For>
              </ul>
            </div>
          )}
        </For>
      </div>
    </div>
  );
}
