// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


import type { Attachment } from "svelte/attachments";

import type { KeyPress } from "./drawn";


export type Walked =
  | { readonly kind: "focus"; readonly at: number }
  | { readonly kind: "close" }
  | { readonly kind: "leave" }
  | { readonly kind: "none" };


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


export interface ListWire {
  readonly role: "menu";
  readonly tabindex: -1;
  readonly "aria-label": string;
  readonly onkeydown: (event: KeyPress) => void;
}


export interface PopupWire {
  readonly id: string;
  readonly onfocusout: (
    event: FocusEvent & { readonly currentTarget: EventTarget & HTMLElement },
  ) => void;
}


export interface HolderWire {
  readonly role: "none";
}


export interface ItemWire {
  readonly type: "button";
  readonly role: "menuitem";
  readonly disabled: boolean;
  readonly onclick: () => void;
  readonly [hold: symbol]: Attachment<HTMLElement>;
}

export interface ItemLook {
  
  readonly key: string;
  
  readonly word: string;
  readonly holder: HolderWire;
  readonly wire: ItemWire;
}


export interface MenuHands {
  
  readonly live: () => readonly HTMLElement[];
  readonly show: () => void;
  readonly close: () => void;
  
  readonly leave: () => void;
}


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


export function popupOf(id: string, hands: Pick<MenuHands, "leave">): PopupWire {
  return {
    id,
    onfocusout: (event) => {
      const next = event.relatedTarget;
      if (!(next instanceof Node && event.currentTarget.contains(next))) hands.leave();
    },
  };
}


export function opening(open: boolean, hands: Pick<MenuHands, "show">): (event: KeyPress) => void {
  return (event) => {
    if (event.key !== "ArrowDown" || open) return;
    event.preventDefault();
    hands.show();
  };
}
