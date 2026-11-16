// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


import { createAttachmentKey } from "svelte/attachments";
import type { Attachment } from "svelte/attachments";


export interface StripTab {
  
  readonly key: string;
  
  readonly label: string;
  
  readonly terminal: boolean;
  
  readonly unsaved: boolean;
  
  readonly front: boolean;
  
  
  readonly href: string | null;
  
  readonly controls: string;
}


export interface StripWords {
  readonly tabs: string;
  readonly unsaved: string;
  readonly closeAll: string;
  readonly closeItem: (name: string) => string;
}


export interface StripHands {
  readonly pick: (at: number) => void;
  readonly close: (at: number) => void;
  readonly closeAll: () => void;
  readonly focus: (at: number) => void;
  readonly hold: (key: string) => Attachment<HTMLElement>;
}


export type KeyPress = Pick<KeyboardEvent, "key" | "preventDefault">;
export type Press = Pick<MouseEvent, "preventDefault">;


export interface ListWire {
  readonly role: "tablist";
  readonly "aria-label": string;
}


export interface TabWire {
  readonly role: "tab";
  readonly href: string | undefined;
  readonly "aria-selected": boolean;
  readonly "aria-controls": string;
  readonly tabindex: 0 | -1;
  readonly onclick: (event: Press) => void;
  readonly onkeydown: (event: KeyPress) => void;
  readonly [hold: symbol]: Attachment<HTMLElement>;
}


export interface CloseWire {
  readonly type: "button";
  readonly tabindex: -1;
  readonly "aria-label": string;
  readonly onclick: () => void;
}


export interface CloseAllWire {
  readonly type: "button";
  readonly "aria-label": string;
  readonly onclick: () => void;
}

export interface TabLook {
  readonly key: string;
  readonly label: string;
  readonly terminal: boolean;
  readonly front: boolean;
  
  readonly unsaved: string | undefined;
  readonly wire: TabWire;
  readonly close: CloseWire;
}


export interface StripLook {
  readonly list: ListWire;
  readonly tabs: readonly TabLook[];
  readonly closeAll: CloseAllWire;
}


export function tabStop(tabs: readonly StripTab[]): number {
  const front = tabs.findIndex((tab) => tab.front);
  return front === -1 ? 0 : front;
}


export function landing(count: number, at: number, key: string): number | undefined {
  const last = count - 1;
  switch (key) {
    case "ArrowLeft":
      return at === 0 ? last : at - 1;
    case "ArrowRight":
      return at === last ? 0 : at + 1;
    case "Home":
      return 0;
    case "End":
      return last;
    default:
      return undefined;
  }
}


const HOLD = createAttachmentKey();


export function lookOf(tabs: readonly StripTab[], words: StripWords, hands: StripHands): StripLook {
  const stop = tabStop(tabs);

  const keys = (event: KeyPress, at: number): void => {
    if (event.key === "Delete") {
      event.preventDefault();
      hands.close(at);
      hands.focus(Math.min(at, tabs.length - 2));
      return;
    }
    const to = landing(tabs.length, at, event.key);
    if (to === undefined) return;
    event.preventDefault();
    hands.pick(to);
    hands.focus(to);
  };

  return {
    list: { role: "tablist", "aria-label": words.tabs },
    tabs: tabs.map((tab, at) => ({
      key: tab.key,
      label: tab.label,
      terminal: tab.terminal,
      front: tab.front,
      unsaved: tab.unsaved ? words.unsaved : undefined,
      wire: {
        role: "tab",
        href: tab.href ?? undefined,
        "aria-selected": tab.front,
        "aria-controls": tab.controls,
        tabindex: at === stop ? 0 : -1,
        onclick: (event) => {
          event.preventDefault();
          hands.pick(at);
        },
        onkeydown: (event) => {
          keys(event, at);
        },
        [HOLD]: hands.hold(tab.key),
      },
      close: {
        type: "button",
        tabindex: -1,
        "aria-label": words.closeItem(tab.label),
        onclick: () => {
          hands.close(at);
        },
      },
    })),
    closeAll: { type: "button", "aria-label": words.closeAll, onclick: hands.closeAll },
  };
}
