// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What a session row's menu look is given (`session_menu.look.svelte`),
// and the wiring the seat (`session_menu.svelte`) builds for it: the menu
// button of `./menu.ts`, whose items the seat decides, and the one field
// the menu turns into for "add a tag" and "rename".

import type { Attachment } from "svelte/attachments";

import { HOLD } from "./drawn";
import { opening, popupOf, walking } from "./menu";
import type { KeyPress } from "./drawn";
import type { ItemLook, ListWire, MenuHands, PopupWire, TriggerWire } from "./menu";

// The bag spread on the form the field stands in.
export interface FormWire {
  readonly onsubmit: (event: SubmitEvent) => void;
}

// The bag spread on the field.
export interface FieldWire {
  readonly "aria-label": string;
  readonly "aria-invalid": boolean;
  readonly "aria-describedby": string;
  readonly maxlength: number;
  readonly value: string;
  readonly oninput: (event: Event & { readonly currentTarget: EventTarget & HTMLInputElement }) => void;
  readonly onkeydown: (event: KeyPress) => void;
  readonly [hold: symbol]: Attachment<HTMLElement>;
}

// The field the menu has turned into, and the grammar hint under it.
export interface NamingLook {
  readonly form: FormWire;
  readonly field: FieldWire;
  readonly hint: { readonly id: string; readonly text: string; readonly wrong: boolean };
}

export interface SessionMenuLook {
  readonly trigger: TriggerWire;
  // Why the button cannot be pressed, already in the person's
  // language; absent while it can.
  readonly why: string | undefined;
  readonly open: boolean;
  readonly popup: PopupWire;
  readonly list: ListWire;
  readonly items: readonly ItemLook[];
  // The field, while "add a tag" or "rename" holds the menu; the items
  // are not drawn then.
  readonly naming: NamingLook | undefined;
}

// One thing the menu does, decided by the seat.
export interface Item {
  readonly id: string;
  readonly word: string;
  readonly act: () => void;
}

// The field's state while it stands in the menu.
export interface Naming {
  readonly label: string;
  readonly hint: string;
  readonly typed: string;
  readonly wrong: boolean;
  readonly max: number;
}

export interface SessionMenuState {
  readonly uid: string;
  readonly open: boolean;
  // Absent: the button is pressable.
  readonly why: string | undefined;
  readonly name: string;
  readonly items: readonly Item[];
  readonly naming: Naming | null;
}

export interface SessionMenuHands extends MenuHands {
  readonly type: (typed: string) => void;
  readonly submit: () => void;
  readonly hold: (key: string) => Attachment<HTMLElement>;
}

export function sessionMenuLookOf(state: SessionMenuState, hands: SessionMenuHands): SessionMenuLook {
  const popup = `${state.uid}-menu`;
  const hint = `${state.uid}-hint`;
  const naming = state.naming;
  return {
    trigger: {
      type: "button",
      "aria-haspopup": "menu",
      "aria-expanded": state.open,
      "aria-controls": popup,
      "aria-label": state.name,
      disabled: state.why !== undefined,
      onclick: () => {
        if (state.open) hands.close();
        else hands.show();
      },
      onkeydown: opening(state.open, hands),
      [HOLD]: hands.hold("trigger"),
    },
    why: state.why,
    open: state.open,
    popup: popupOf(popup, hands),
    list: { role: "menu", tabindex: -1, "aria-label": state.name, onkeydown: walking(hands) },
    items: state.items.map((item) => ({
      key: item.id,
      word: item.word,
      holder: { role: "none" },
      wire: { type: "button", role: "menuitem", disabled: false, onclick: item.act, [HOLD]: hands.hold(item.id) },
    })),
    naming:
      naming === null
        ? undefined
        : {
            form: {
              onsubmit: (event) => {
                event.preventDefault();
                hands.submit();
              },
            },
            field: {
              "aria-label": naming.label,
              "aria-invalid": naming.wrong,
              "aria-describedby": hint,
              maxlength: naming.max,
              value: naming.typed,
              oninput: (event) => {
                hands.type(event.currentTarget.value);
              },
              onkeydown: (event) => {
                if (event.key !== "Escape") return;
                event.preventDefault();
                hands.close();
              },
              [HOLD]: hands.hold("field"),
            },
            hint: { id: hint, text: naming.hint, wrong: naming.wrong },
          },
  };
}
