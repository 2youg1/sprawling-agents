// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


import type { Attachment } from "svelte/attachments";

import type { Pane, Side, Workbench } from "../../core/workbench";
import { HOLD } from "./drawn";
import { opening, popupOf, walking } from "./menu";
import type { ItemLook, ListWire, MenuHands, PopupWire, TriggerWire } from "./menu";


export type Arranged = "menu" | "words";

export interface PaneMenuLook {
  
  readonly label: string;
  readonly arranged: Arranged;
  readonly open: boolean;
  readonly trigger: TriggerWire;
  readonly popup: PopupWire;
  readonly list: ListWire;
  readonly items: readonly ItemLook[];
}


export interface PaneMenuWords {
  readonly label: string;
  readonly arrange: string;
  readonly left: string;
  readonly right: string;
}

export interface PaneMenuState {
  readonly pane: Pane;
  readonly bench: Workbench;
  readonly arranged: Arranged;
  readonly open: boolean;
  
  readonly uid: string;
}

export interface PaneMenuHands extends MenuHands {
  readonly move: (side: Side) => void;
  
  
  readonly hold: (key: string) => Attachment<HTMLElement>;
}

const SIDES: readonly Side[] = ["left", "right"];


export function possible(bench: Workbench, pane: Pane, side: Side): boolean {
  const at = bench.findIndex((column) => column.pane === pane);
  return side === "left" ? at > 0 : at >= 0 && at < bench.length - 1;
}

export function paneMenuLookOf(
  state: PaneMenuState,
  words: PaneMenuWords,
  hands: PaneMenuHands,
): PaneMenuLook {
  const popup = `${state.uid}-menu`;
  return {
    label: words.label,
    arranged: state.arranged,
    open: state.open,
    trigger: {
      type: "button",
      "aria-haspopup": "menu",
      "aria-expanded": state.open,
      "aria-controls": popup,
      "aria-label": words.arrange,
      disabled: false,
      onclick: () => {
        if (state.open) hands.close();
        else hands.show();
      },
      onkeydown: opening(state.open, hands),
      [HOLD]: hands.hold("trigger"),
    },
    popup: popupOf(popup, hands),
    list: { role: "menu", tabindex: -1, "aria-label": words.label, onkeydown: walking(hands) },
    items: SIDES.map((side) => ({
      key: side,
      word: side === "left" ? words.left : words.right,
      holder: { role: "none" },
      wire: {
        type: "button",
        role: "menuitem",
        disabled: !possible(state.bench, state.pane, side),
        onclick: () => {
          hands.move(side);
        },
        [HOLD]: hands.hold(side),
      },
    })),
  };
}
