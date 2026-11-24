// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { ElementRef } from "../../core/element_ref";

import { HOLD } from "./drawn";
import { opening, popupOf, walking } from "./menu";
import type { KeyPress } from "./drawn";
import type { ItemLook, ListWire, MenuHands, PopupWire, TriggerWire } from "./menu";

export interface FormWire {
  readonly onsubmit: (event: SubmitEvent) => void;
}

export interface FieldWire {
  readonly "aria-label": string;
  readonly "aria-invalid": boolean;
  readonly "aria-describedby": string;
  readonly maxlength: number;
  readonly value: string;
  readonly oninput: (
    event: Event & { readonly currentTarget: EventTarget & HTMLInputElement },
  ) => void;
  readonly onkeydown: (event: KeyPress & Pick<KeyboardEvent, "isComposing">) => void;
  readonly [hold: symbol]: ElementRef<HTMLElement>;
}

export interface NamingLook {
  readonly form: FormWire;
  readonly field: FieldWire;
  readonly hint: { readonly id: string; readonly text: string; readonly wrong: boolean };
}

export interface SessionMenuLook {
  readonly trigger: TriggerWire;
  readonly why: string | undefined;
  readonly open: boolean;
  readonly popup: PopupWire;
  readonly list: ListWire;
  readonly items: readonly ItemLook[];
  readonly naming: NamingLook | undefined;
}

export interface Item {
  readonly id: string;
  readonly word: string;
  readonly act: () => void;
}

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
  readonly why: string | undefined;
  readonly name: string;
  readonly items: readonly Item[];
  readonly naming: Naming | null;
}

export interface SessionMenuHands extends MenuHands {
  readonly type: (typed: string) => void;
  readonly submit: () => void;
  readonly hold: (key: string) => ElementRef<HTMLElement>;
}

export function sessionMenuLookOf(
  state: SessionMenuState,
  hands: SessionMenuHands,
): SessionMenuLook {
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
      wire: {
        type: "button",
        role: "menuitem",
        disabled: false,
        onclick: item.act,
        [HOLD]: hands.hold(item.id),
      },
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
                if (event.key !== "Escape" || event.isComposing) return;
                event.preventDefault();
                hands.close();
              },
              [HOLD]: hands.hold("field"),
            },
            hint: { id: hint, text: naming.hint, wrong: naming.wrong },
          },
  };
}
