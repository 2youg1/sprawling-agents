// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createSignal, createUniqueId, flush, For, Show } from "solid-js";

import { fill, say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { moved, type Pane, type Side } from "../../core/workbench";
import { ui } from "../../ui";
import { opening, popupOf, walking, type MenuHands } from "./menu";
import { possible, type Arranged } from "./pane_menu";

export interface PaneMenuProps {
  readonly pane: Pane;
  readonly label: string;
  readonly arranged: Arranged;
}

const SIDES: readonly Side[] = ["left", "right"];

export default function PaneMenu(props: PaneMenuProps) {
  const u = ui();
  const lang = storeValue(u.lang);
  const bench = storeValue(u.prefs.workbench);
  const popup = `${createUniqueId()}-menu`;
  const [open, setOpen] = createSignal(false);
  const buttons: Partial<Record<Side, HTMLButtonElement>> = {};
  let trigger: HTMLButtonElement | undefined;

  function live(): HTMLButtonElement[] {
    return SIDES.flatMap((side) => {
      const button = buttons[side];
      return button?.isConnected && possible(bench(), props.pane, side) ? [button] : [];
    });
  }

  const hands: MenuHands = {
    live,
    show() {
      setOpen(true);
      queueMicrotask(() => {
        flush();
        if (open()) live()[0]?.focus();
      });
    },
    close() {
      setOpen(false);
      queueMicrotask(() => {
        flush();
        if (!open() && trigger?.isConnected) trigger.focus();
      });
    },
    leave() {
      setOpen(false);
    },
  };

  function move(side: Side): void {
    u.prefs.setWorkbench(moved(bench(), props.pane, side));
    hands.close();
  }

  return (
    <Show
      when={props.arranged === "menu"}
      fallback={
        <h2 class="flex h-control shrink-0 items-center text-note text-text-faint">
          {props.label}
        </h2>
      }
    >
      <div class="relative flex h-control shrink-0 items-center">
        <h2 class="contents">
          <button
            data-control=""
            ref={(node) => {
              trigger = node;
            }}
            type="button"
            aria-haspopup="menu"
            aria-expanded={open() ? "true" : "false"}
            aria-controls={popup}
            aria-label={fill(say(lang(), "world_arrange"), { pane: props.label })}
            class="-mx-snug flex h-control-sm items-center rounded-control px-snug text-note text-text-faint transition-[background-color,color,scale] ease-leave hover:wash hover:text-text hover:ease-arrive active:scale-(--scale-press) active:ease-arrive aria-expanded:wash aria-expanded:text-text still:transition-none still:active:scale-100"
            onClick={() => {
              if (open()) hands.close();
              else hands.show();
            }}
            onKeyDown={(event) => opening(open(), hands)(event)}
          >
            {props.label}
          </button>
        </h2>
        <Show when={open()}>
          <ul
            id={popup}
            role="menu"
            tabindex={-1}
            aria-label={props.label}
            class="absolute top-full left-0 z-(--layer-popover) flex w-max min-w-[20ch] max-w-[36ch] max-h-palette flex-col overflow-y-auto rounded-card bg-raised p-tight shadow-float"
            onFocusOut={popupOf(popup, hands).onfocusout}
            onKeyDown={walking(hands)}
          >
            <For each={SIDES}>
              {(side) => (
                <li role="none">
                  <button
                    data-control=""
                    ref={(node) => {
                      buttons[side] = node;
                    }}
                    type="button"
                    role="menuitem"
                    disabled={!possible(bench(), props.pane, side)}
                    class="flex min-h-control-sm w-full items-center rounded-control px-snug py-tight text-left text-note text-pretty text-text transition-[background-color,color] ease-leave hover:wash hover:ease-arrive focus-visible:wash active:wash-strong active:ease-arrive disabled:text-text-disabled"
                    onClick={() => move(side)}
                  >
                    {say(lang(), side === "left" ? "world_move_left" : "world_move_right")}
                  </button>
                </li>
              )}
            </For>
          </ul>
        </Show>
      </div>
    </Show>
  );
}
