// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The world layer's two menu buttons - a workbench pane's label
// (`pane_menu.svelte`) and a session row's actions (`session_menu.svelte`)
// - are one APG Menu Button (client/spec/Views/Workspace.lean §7-11):
// Enter, Space or Down opens the menu on its first item, Up and Down walk
// the items and wrap at both ends, Escape closes with the focus back on
// the button, and Tab closes and lets the focus go on in document order.
// This file is that key table, once, and the wire bags a menu's look
// spreads; each seat owns its own items and what picking one does.

import type { Attachment } from "svelte/attachments";

import type { KeyPress } from "./drawn";

// What a key pressed inside an open menu does: move the focus to the
// live item at `at`, close with the focus back on the button, or let the
// menu go because the focus is leaving it.
export type Walked =
  | { readonly kind: "focus"; readonly at: number }
  | { readonly kind: "close" }
  | { readonly kind: "leave" }
  | { readonly kind: "none" };

// `count` live items, the focus on the one at `now` (-1: on none of
// them, as when the menu itself holds the focus). Up from none lands on
// the last item, as Down from none lands on the first.
export function walked(count: number, now: number, key: string): Walked {
  switch (key) {
    case "ArrowDown":
      return count === 0 ? { kind: "none" } : { kind: "focus", at: (now + 1) % count };
    case "ArrowUp":
      return count === 0 ? { kind: "none" } : { kind: "focus", at: now <= 0 ? count - 1 : now - 1 };
    case "Escape":
      return { kind: "close" };
    case "Tab":
      return { kind: "leave" };
    default:
      return { kind: "none" };
  }
}

// The bag spread on the menu button.
export interface TriggerWire {
  readonly type: "button";
  readonly "aria-haspopup": "menu";
  readonly "aria-expanded": boolean;
  readonly "aria-controls": string;
  readonly "aria-label": string;
  readonly disabled: boolean;
  readonly onclick: () => void;
  readonly onkeydown: (event: KeyPress) => void;
  readonly [hold: symbol]: Attachment<HTMLElement>;
}

// The bag spread on the element that holds the open menu's items.
export interface ListWire {
  readonly role: "menu";
  readonly tabindex: -1;
  readonly "aria-label": string;
  readonly onkeydown: (event: KeyPress) => void;
}

// The bag spread on the box the menu opens in, which the button's
// `aria-controls` names; the focus leaving it closes the menu.
export interface PopupWire {
  readonly id: string;
  readonly onfocusout: (event: FocusEvent & { readonly currentTarget: EventTarget & HTMLElement }) => void;
}

// The `<li>` around an item takes no role of its own, so a screen
// reader hears a menu of items rather than a list.
export interface HolderWire {
  readonly role: "none";
}

// The bag spread on one item's button.
export interface ItemWire {
  readonly type: "button";
  readonly role: "menuitem";
  readonly disabled: boolean;
  readonly onclick: () => void;
  readonly [hold: symbol]: Attachment<HTMLElement>;
}

export interface ItemLook {
  // Unique within the menu; a key for `#each`.
  readonly key: string;
  // Already in the person's language.
  readonly word: string;
  readonly holder: HolderWire;
  readonly wire: ItemWire;
}

// What a seat lends the wiring: the drawn button and items, and what an
// open or a close does to the seat's own state.
export interface MenuHands {
  // The live items' elements, in order; a disabled item is not live.
  readonly live: () => readonly HTMLElement[];
  readonly show: () => void;
  readonly close: () => void;
  // The focus is leaving the menu: shut it without taking the focus.
  readonly leave: () => void;
}

// The key handler on an open menu's list.
export function walking(hands: MenuHands): (event: KeyPress) => void {
  return (event) => {
    const live = hands.live();
    const step = walked(
      live.length,
      live.findIndex((entry) => entry === document.activeElement),
      event.key,
    );
    switch (step.kind) {
      case "focus":
        event.preventDefault();
        live[step.at]?.focus();
        return;
      case "close":
        event.preventDefault();
        hands.close();
        return;
      case "leave":
        hands.leave();
        return;
      case "none":
        return;
    }
  };
}

// The box the menu opens in: the focus leaving it lets the menu go.
export function popupOf(id: string, hands: Pick<MenuHands, "leave">): PopupWire {
  return {
    id,
    onfocusout: (event) => {
      const next = event.relatedTarget;
      if (!(next instanceof Node && event.currentTarget.contains(next))) hands.leave();
    },
  };
}

// The key handler on the button: Down opens a closed menu, as Enter and
// Space do through the click.
export function opening(open: boolean, hands: Pick<MenuHands, "show">): (event: KeyPress) => void {
  return (event) => {
    if (event.key !== "ArrowDown" || open) return;
    event.preventDefault();
    hands.show();
  };
}
