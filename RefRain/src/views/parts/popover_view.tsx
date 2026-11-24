// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { JSX } from "@solidjs/web";
import {
  createEffect,
  createMemo,
  createSignal,
  createUniqueId,
  For,
  onCleanup,
  Show,
} from "solid-js";
import { say, type Key } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import { revealIn, roomOn, sideFor, type Side } from "./layer";
import type { PopoverBinding, PopoverColumn, PopoverRow } from "./popover";
import {
  keysOf,
  listId,
  rowId,
  type Keyed,
  type Layout,
  type Place,
  type PopoverActions,
  type PopoverView,
} from "./popover_keys";
import "./popover_view.css";

export interface PopoverProps {
  readonly label: Key;
  readonly columns: readonly PopoverColumn[];
  readonly layout?: Layout;
  readonly onApply: (column: PopoverColumn, row: PopoverRow) => void;
  readonly onClose: () => void;
  readonly bind?: ((binding: PopoverBinding) => void) | undefined;
  readonly onCursorChange?: ((rowId: string | null) => void) | undefined;
  readonly onCursorRow?: ((row: PopoverRow | null) => void) | undefined;
  readonly row?: ((row: PopoverRow) => JSX.Element) | undefined;
  readonly header?: (() => JSX.Element) | undefined;
  readonly presentation?: PopoverPresentation | undefined;
}

export interface PopoverSection {
  readonly heading: string;
  readonly aside: string | undefined;
  readonly empty: string;
  readonly spare: number;
  readonly band: boolean;
  readonly scrolls: boolean;
  readonly heard: Readonly<Record<string, string>>;
}

export interface PopoverPresentation {
  readonly sections: Readonly<Record<string, PopoverSection>>;
  readonly before?: ((column: PopoverColumn) => JSX.Element) | undefined;
  readonly footer?: (() => JSX.Element) | undefined;
}

const BAND_HEADING =
  "flex h-control-sm items-center justify-between gap-base px-snug text-note text-text-faint";
const BAND_NOTE = "line-clamp-2 min-h-[2lh] px-snug text-note text-text-quiet";

export default function Popover(props: PopoverProps) {
  const lang = storeValue(ui().lang);
  const uid = createUniqueId();
  const [column, setColumn] = createSignal(0);
  const [cursor, setCursor] = createSignal(0);
  const [side, setSide] = createSignal<Side>("above");
  const [room, setRoom] = createSignal<number | null>(null);
  const [dialog, setDialog] = createSignal<HTMLDivElement>();
  const [lists, setLists] = createSignal<ReadonlyMap<string, HTMLUListElement>>(new Map());
  const place = createMemo<Place>(() => {
    const at = Math.min(column(), Math.max(0, props.columns.length - 1));
    const size = props.columns.at(at)?.rows.length ?? 0;
    return { column: at, cursor: Math.min(cursor(), Math.max(0, size - 1)) };
  });
  const pane = createMemo(() => props.columns.at(place().column));
  const here = createMemo(() => pane()?.rows ?? []);
  const activeList = createMemo(() => {
    const id = pane()?.id;
    return id === undefined ? undefined : lists().get(id);
  });
  const activeId = createMemo(() =>
    here().length === 0 ? null : rowId(uid, place().column, place().cursor),
  );
  const view = createMemo<PopoverView>(() => ({
    seat: uid,
    layout: props.layout ?? "content",
    columns: props.columns,
    place: place(),
    holder: props.bind === undefined ? "list" : "caller",
    side: side(),
    room: room(),
    title: props.label,
    say: (key) => say(lang(), key),
  }));
  const actions: PopoverActions = {
    place: (to) => {
      setColumn(to.column);
      setCursor(to.cursor);
    },
    apply: (pane, row) => {
      props.onApply(pane, row);
    },
    close: () => {
      props.onClose();
    },
  };
  const keys = (key: Keyed): boolean => keysOf(view(), actions)(key);
  function pointColumn(id: string): void {
    const at = props.columns.findIndex((pane) => pane.id === id);
    if (at < 0) return;
    setColumn(at);
    setCursor(Math.max(0, props.columns.at(at)?.rows.findIndex((row) => row.chosen === true) ?? 0));
  }

  let measured = false;
  let opener: HTMLElement | null = null;
  let captured = false;
  createEffect(dialog, (sheet) => {
    if (measured || sheet === undefined) return;
    measured = true;
    const picked = sideFor(sheet, "above");
    setSide(picked);
    setRoom(roomOn(sheet, picked));
  });
  createEffect(
    () => ({ target: activeList(), external: props.bind !== undefined }),
    ({ target, external }) => {
      if (external || target === undefined) return;
      if (!captured) {
        captured = true;
        const held = document.activeElement;
        opener = held instanceof HTMLElement ? held : null;
      }
      target.focus({ preventScroll: true });
    },
  );
  onCleanup(() => {
    if (props.bind === undefined) opener?.focus();
  });
  createEffect(
    () => ({ bind: props.bind, controls: props.columns.map((_pane, at) => listId(uid, at)) }),
    ({ bind, controls }) => {
      bind?.({ keys, pointColumn, controls });
    },
  );
  createEffect(
    () => ({ id: activeId(), list: activeList() }),
    ({ id, list }) => {
      if (id === null || list === undefined) return;
      const selected = document.getElementById(id);
      if (selected !== null) revealIn(list, selected);
    },
  );
  createEffect(activeId, (id) => {
    props.onCursorChange?.(id);
  });
  createEffect(
    () => here().at(place().cursor) ?? null,
    (row) => {
      props.onCursorRow?.(row);
    },
  );

  function Options(propsForPane: { readonly pane: PopoverColumn; readonly index: number }) {
    const id = propsForPane.pane.id;
    const section = createMemo(() => props.presentation?.sections[id]);
    const heading = () => section()?.heading ?? say(lang(), propsForPane.pane.label);
    const note = () => {
      const chosen = propsForPane.index === place().column ? here().at(place().cursor) : undefined;
      return (chosen ?? propsForPane.pane.rows.find((row) => row.chosen === true))?.secondary ?? "";
    };
    const spare = createMemo(() =>
      Array.from(
        {
          length: Math.max(
            0,
            (section()?.spare ?? 0) - Number(propsForPane.pane.rows.length === 0),
          ),
        },
        (_unused, index) => index,
      ),
    );
    let current: HTMLUListElement | undefined;
    const focused = createMemo(
      () => props.bind === undefined && propsForPane.index === place().column,
    );
    function hold(node: HTMLUListElement): void {
      current = node;
      setLists((held) => new Map(held).set(id, node));
    }
    onCleanup(() => {
      setLists((held) => {
        if (held.get(id) !== current) return held;
        const next = new Map(held);
        next.delete(id);
        return next;
      });
    });
    return (
      <div class={["flex min-w-0 flex-col", section() && "gap-tight"]}>
        {props.presentation?.before?.(propsForPane.pane)}
        <div
          class={
            section() === undefined ? "mb-tight px-snug text-note text-text-faint" : BAND_HEADING
          }
        >
          <span class="min-w-0 truncate">{heading()}</span>
          <Show when={section()?.aside}>{(aside) => <span class="shrink-0">{aside()}</span>}</Show>
        </div>
        <ul
          class={[
            section() === undefined
              ? "max-h-palette overflow-y-auto"
              : section()?.band === true
                ? "flex gap-tight"
                : "flex flex-col",
            section()?.scrolls === true && "max-h-[calc(var(--spacing-touch)*6)] overflow-y-auto",
          ]}
          role="listbox"
          ref={hold}
          id={listId(uid, propsForPane.index)}
          aria-label={heading()}
          tabindex={focused() ? 0 : -1}
          aria-activedescendant={focused() ? (activeId() ?? undefined) : undefined}
          onKeyDown={(event) => {
            if (!event.isComposing && keys(event)) event.preventDefault();
          }}
        >
          <For
            each={propsForPane.pane.rows}
            keyed={(row) => row.id}
            fallback={
              <li
                role="presentation"
                class={
                  section() === undefined
                    ? "px-snug py-tight text-note text-text-faint"
                    : "flex h-touch items-center px-base text-note text-text-faint"
                }
              >
                {section()?.empty ?? say(lang(), "part_no_match")}
              </li>
            }
          >
            {(row, at) => {
              const cursor = createMemo(
                () => propsForPane.index === place().column && at() === place().cursor,
              );
              return (
                <li
                  role="option"
                  id={rowId(uid, propsForPane.index, at())}
                  aria-label={section()?.heard[row().id]}
                  aria-selected={row().chosen === true ? "true" : "false"}
                  data-control=""
                  onMouseEnter={() => {
                    actions.place({ column: propsForPane.index, cursor: at() });
                  }}
                  onClick={() => {
                    actions.apply(propsForPane.pane, row());
                  }}
                  class={[
                    "relative flex cursor-pointer items-center justify-between gap-snug rounded-control px-snug py-tight text-body transition-[background-color,color,scale] ease-leave active:scale-(--scale-press) active:ease-arrive still:active:scale-100",
                    section() &&
                      "h-touch min-w-0 shrink-0 px-base outline-1 -outline-offset-1 outline-transparent",
                    section()?.band === true && "flex-1 justify-center",
                    row().chosen === true
                      ? "popover-applied wash-strong text-text active:bg-raised-hover"
                      : cursor()
                        ? "bg-raised-hover text-text-quiet hover:text-text hover:ease-arrive active:wash-strong"
                        : "text-text-quiet hover:bg-raised-hover hover:text-text hover:ease-arrive active:wash-strong",
                    cursor() && "popover-cursor",
                  ]}
                >
                  <Show
                    when={props.row}
                    fallback={
                      <>
                        <span
                          class={
                            section() === undefined
                              ? "min-w-0 max-w-[28ch] wrap-anywhere text-pretty"
                              : section()?.band === true || row().secondary === undefined
                                ? "min-w-0 truncate"
                                : "max-w-[60%] shrink-0 truncate"
                          }
                        >
                          {row().label}
                        </span>
                        <Show when={section()?.band === true ? undefined : row().secondary}>
                          {(secondary) => (
                            <span
                              class={
                                section() === undefined
                                  ? "min-w-0 max-w-[20ch] line-clamp-2 text-note text-text-faint"
                                  : "min-w-0 truncate text-note text-text-faint"
                              }
                            >
                              {secondary()}
                            </span>
                          )}
                        </Show>
                      </>
                    }
                  >
                    {(draw) => draw()(row())}
                  </Show>
                </li>
              );
            }}
          </For>
          <For each={spare()}>
            {() => <li role="presentation" aria-hidden="true" class="h-touch shrink-0" />}
          </For>
        </ul>
        <Show when={section()?.band === true}>
          <p class={BAND_NOTE}>{note()}</p>
        </Show>
      </div>
    );
  }

  const cap = createMemo(() => {
    const height = room();
    if (height === null) return undefined;
    const fitted = `calc(${String(height)}px - var(--spacing-snug) * 2)`;
    return props.presentation === undefined ? fitted : `min(85dvh, ${fitted})`;
  });
  return (
    <div
      role="dialog"
      aria-label={say(lang(), props.label)}
      ref={(node) => {
        setDialog(node);
      }}
      style={{ "max-height": cap() }}
      class={[
        "absolute z-(--layer-popover) max-w-full overflow-y-auto rounded-panel border border-edge-panel bg-raised shadow-float",
        props.presentation === undefined
          ? "left-0 w-max min-w-full p-snug"
          : "right-0 flex max-h-[85dvh] w-[34rem] flex-col gap-snug py-snug pl-snug",
        side() === "above" ? "bottom-full mb-snug rise" : "top-full mt-snug drop",
      ]}
    >
      {props.header?.()}
      <div
        class={[
          "gap-snug",
          props.presentation !== undefined
            ? "flex flex-col"
            : props.layout === "equal"
              ? "grid auto-cols-fr grid-flow-col"
              : "flex",
        ]}
      >
        <For each={props.columns} keyed={(pane) => pane.id}>
          {(pane, at) => <Options pane={pane()} index={at()} />}
        </For>
      </div>
      {props.presentation?.footer?.()}
    </div>
  );
}
