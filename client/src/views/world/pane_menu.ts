// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What a workbench pane's label look is given (`pane_menu.look.svelte`),
// and the wiring the seat (`pane_menu.svelte`) builds for it: the menu
// button of client/spec/Views/Workspace.lean §7-11 that moves the pane one
// place left or right. An item that would move the pane past the edge of
// the workbench is drawn but cannot be picked, and the arrows skip it.

import type { Attachment } from "svelte/attachments";

import type { Pane, Side, Workbench } from "../../core/workbench";
import { HOLD } from "./drawn";
import { opening, popupOf, walking } from "./menu";
import type { ItemLook, ListWire, MenuHands, PopupWire, TriggerWire } from "./menu";

// Whether the label is a control: in the blend tier the world layer
// takes no input, and its labels are only words.
export type Arranged = "menu" | "words";

export interface PaneMenuLook {
  // The pane's name, already in the person's language.
  readonly label: string;
  readonly arranged: Arranged;
  readonly open: boolean;
  readonly trigger: TriggerWire;
  readonly popup: PopupWire;
  readonly list: ListWire;
  readonly items: readonly ItemLook[];
}

// The words the look and the bags carry, already in the person's
// language: the button's accessible name and the two items.
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
  // The seat's id, which the popup's id is made from.
  readonly uid: string;
}

export interface PaneMenuHands extends MenuHands {
  readonly move: (side: Side) => void;
  // The attachment that records the element drawn for `key` ("trigger",
  // or an item's side); the same one for the same key on every call.
  readonly hold: (key: string) => Attachment<HTMLElement>;
}

const SIDES: readonly Side[] = ["left", "right"];

// The sides the pane can move to from where it stands in `bench`.
export function possible(bench: Workbench, pane: Pane, side: Side): boolean {
  const at = bench.findIndex((column) => column.pane === pane);
  return side === "left" ? at > 0 : at >= 0 && at < bench.length - 1;
}

export function paneMenuLookOf(state: PaneMenuState, words: PaneMenuWords, hands: PaneMenuHands): PaneMenuLook {
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
