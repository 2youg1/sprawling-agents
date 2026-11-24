// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, createSignal, createUniqueId, For, onSettled, Show } from "solid-js";
import { say } from "../../core/lang";
import { collapsedOf } from "../../core/session_tree";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { Seat } from "../modal";
import { askStretches } from "./stretches";
import { switchItems, switcherLookOf, type SwitcherLook } from "./switcher";

export function SwitcherFrame(look: SwitcherLook) {
  return (
    <div class="flex min-h-0 flex-col gap-snug">
      <input
        class="h-control rounded-control bg-page px-base text-body text-text"
        role={look.box.role}
        aria-autocomplete={look.box["aria-autocomplete"]}
        aria-expanded={look.box["aria-expanded"] ? "true" : "false"}
        aria-controls={look.box["aria-controls"]}
        aria-activedescendant={look.box["aria-activedescendant"]}
        aria-label={look.box["aria-label"]}
        placeholder={look.box["aria-label"]}
        value={look.box.value}
        onInput={(event) => {
          look.box.oninput(event);
        }}
        onKeyDown={(event) => {
          look.box.onkeydown(event);
        }}
      />
      <Show
        when={look.empty}
        fallback={
          <div
            id={look.tree.id}
            role={look.tree.role}
            aria-label={look.tree["aria-label"]}
            class="max-h-palette overflow-y-auto"
          >
            <For each={look.rows} keyed={(row) => row.key}>
              {(row) => (
                <div
                  id={row().wire.id}
                  role={row().wire.role}
                  aria-level={row().wire["aria-level"]}
                  aria-selected={row().wire["aria-selected"] ? "true" : "false"}
                  aria-expanded={
                    row().wire["aria-expanded"] === undefined
                      ? undefined
                      : row().wire["aria-expanded"]
                        ? "true"
                        : "false"
                  }
                  aria-disabled={row().wire["aria-disabled"] ? "true" : "false"}
                  onMouseEnter={() => {
                    row().wire.onmouseenter();
                  }}
                  onClick={() => {
                    row().wire.onclick();
                  }}
                  class={[
                    "flex w-full cursor-pointer items-center justify-between gap-snug rounded-control px-base py-snug text-left text-body transition-[background-color,color] ease-leave aria-disabled:text-text-disabled",
                    row().wire["aria-selected"]
                      ? "bg-raised-hover ease-arrive"
                      : "hover:bg-raised-hover hover:ease-arrive active:wash-strong active:ease-arrive",
                  ]}
                  style={{
                    "padding-inline-start": `calc(${String(row().depth)} * var(--spacing-base) + var(--spacing-base))`,
                  }}
                >
                  <span class="max-w-full shrink-0 truncate font-mono">{row().label}</span>
                  <span class="min-w-0 shrink-1000 truncate text-note text-text-faint">
                    {row().hint}
                  </span>
                </div>
              )}
            </For>
          </div>
        }
      >
        {(empty) => <p class="px-base py-snug text-note text-text-faint">{empty()}</p>}
      </Show>
    </div>
  );
}

export interface SessionSwitcherProps {
  readonly onClose: () => void;
  readonly seat?: Seat | undefined;
}

export default function SessionSwitcher(props: SessionSwitcherProps) {
  const u = ui();
  const lang = storeValue(u.lang);
  const stretches = storeValue(askStretches(u));
  const [query, setQuery] = createSignal("");
  const [cursor, setCursor] = createSignal(0);
  const [overrides, setOverrides] = createSignal<Readonly<Record<string, boolean>>>({});
  const collapsed = createMemo(() => collapsedOf(stretches().all, overrides()));
  const rows = createMemo(() =>
    switchItems(stretches().all, query(), collapsed(), lang(), u.now()),
  );
  const list = `${createUniqueId()}-tree`;
  const specimen = () => props.seat === "specimen";
  let dialog: HTMLDialogElement | undefined;

  const look = createMemo(() =>
    switcherLookOf(
      {
        query: query(),
        cursor: cursor(),
        collapsed: collapsed(),
        rows: rows(),
        list,
        lang: lang(),
      },
      {
        write: (line) => {
          setQuery(line);
          setCursor(0);
        },
        move: setCursor,
        fold: (next) => {
          setOverrides(
            Object.fromEntries(
              rows()
                .filter((row) => row.children)
                .map((row) => [row.key, next.has(row.key)]),
            ),
          );
        },
        open: (row) => {
          u.go(row.to);
          props.onClose();
        },
        hover: setCursor,
      },
    ),
  );
  onSettled(() => {
    const sheet = dialog;
    if (specimen() || sheet === undefined) return;
    const before = document.activeElement;
    sheet.showModal();
    return () => {
      sheet.close();
      const lost = document.activeElement === document.body || document.activeElement === null;
      if (lost && before instanceof HTMLElement && before.isConnected) before.focus();
    };
  });

  return (
    <dialog
      ref={(node) => {
        dialog = node;
      }}
      open={specimen() ? true : undefined}
      aria-label={say(lang(), "world_session_switcher")}
      onCancel={(event) => {
        event.preventDefault();
        if (!specimen()) props.onClose();
      }}
      onClick={(event) => {
        if (!specimen() && event.target === event.currentTarget) props.onClose();
      }}
      class={[
        "m-auto mt-section hidden w-full max-w-measure rounded-panel bg-raised p-0 opacity-0 shadow-sheet",
        "transition-[opacity,display,overlay] transition-discrete duration-panel ease-leave",
        "open:block open:opacity-100 open:ease-arrive starting:open:opacity-0 still:transition-none",
        "backdrop:bg-transparent backdrop:backdrop-brightness-50",
        specimen() && "static",
      ]}
    >
      <div class="flex flex-col gap-snug p-snug">
        <SwitcherFrame {...look()} />
      </div>
    </dialog>
  );
}
