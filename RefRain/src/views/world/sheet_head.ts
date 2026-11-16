// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Attachment } from "svelte/attachments";

import type { Pane } from "../../core/workbench";
import { HOLD } from "./drawn";
import type { KeyPress } from "./drawn";

export function landing(count: number, at: number, key: string): number | undefined {
  const last = count - 1;
  if (last < 0) return undefined;
  switch (key) {
    case "ArrowRight":
      return at >= last ? 0 : at + 1;
    case "ArrowLeft":
      return at <= 0 ? last : at - 1;
    case "Home":
      return 0;
    case "End":
      return last;
    default:
      return undefined;
  }
}

export interface BackWire {
  readonly type: "button";
  readonly "aria-label": string;
  readonly onclick: () => void;
}

export interface StripWire {
  readonly role: "tablist";
  readonly "aria-label": string;
}

export interface TabWire {
  readonly type: "button";
  readonly role: "tab";
  readonly id: string;
  readonly "aria-controls": string;
  readonly "aria-selected": boolean;
  readonly tabindex: 0 | -1;
  readonly onclick: () => void;
  readonly onkeydown: (event: KeyPress) => void;
  readonly [hold: symbol]: Attachment<HTMLElement>;
}

export interface TabLook {
  readonly key: Pane;

  readonly label: string;
  readonly shown: boolean;
  readonly wire: TabWire;
}

export interface SheetHeadLook {
  readonly back: BackWire;
  readonly strip: StripWire;
  readonly tabs: readonly TabLook[];
}

export interface SheetHeadInput {
  readonly tabs: readonly { readonly pane: Pane; readonly label: string }[];
  readonly shown: Pane;

  readonly prefix: string;
  readonly words: { readonly back: string; readonly strip: string };
}

export interface SheetHeadHands {
  readonly leave: () => void;
  readonly show: (pane: Pane) => void;
  readonly focus: (pane: Pane) => void;
  readonly hold: (pane: Pane) => Attachment<HTMLElement>;
}

export function sheetHeadOf(input: SheetHeadInput, hands: SheetHeadHands): SheetHeadLook {
  const { tabs, shown, prefix } = input;
  const at = tabs.findIndex((tab) => tab.pane === shown);
  return {
    back: { type: "button", "aria-label": input.words.back, onclick: hands.leave },
    strip: { role: "tablist", "aria-label": input.words.strip },
    tabs: tabs.map((tab) => ({
      key: tab.pane,
      label: tab.label,
      shown: tab.pane === shown,
      wire: {
        type: "button",
        role: "tab",
        id: `${prefix}-tab-${tab.pane}`,
        "aria-controls": `${prefix}-${tab.pane}`,
        "aria-selected": tab.pane === shown,
        tabindex: tab.pane === shown ? 0 : -1,
        onclick: () => {
          hands.show(tab.pane);
        },
        onkeydown: (event) => {
          const to = landing(tabs.length, at, event.key);
          const next = to === undefined ? undefined : tabs[to];
          if (next === undefined) return;
          event.preventDefault();
          hands.show(next.pane);
          hands.focus(next.pane);
        },
        [HOLD]: hands.hold(tab.pane),
      },
    })),
  };
}
