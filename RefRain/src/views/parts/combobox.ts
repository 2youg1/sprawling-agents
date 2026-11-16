// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


import { createAttachmentKey } from "svelte/attachments";
import type { Attachment } from "svelte/attachments";

import type { Side } from "./layer";

export interface Choice {
  readonly value: string;
  
  readonly label: string;
  
  readonly note?: string;
  
  
  readonly face?: string;
}

export interface ComboboxProps {
  
  readonly label: string;
  
  
  readonly placeholder: string;
  
  readonly empty: string;
  readonly choices: readonly Choice[];
  
  readonly value: string | null;
  
  readonly onPick: (value: string) => void;
  
  
  readonly starts?: "open" | "closed";
}


export function matching(choices: readonly Choice[], query: string): readonly Choice[] {
  const needle = query.trim().toLowerCase();
  if (needle === "") return choices;
  return choices.filter((choice) =>
    `${choice.label} ${choice.note ?? ""} ${choice.value}`.toLowerCase().includes(needle),
  );
}


export type Return = "opener" | "leave";


export type Answer =
  | { readonly kind: "cursor"; readonly at: number }
  | { readonly kind: "take" }
  | { readonly kind: "close"; readonly focus: Return }
  | { readonly kind: "pass" };


export type Origin = "trigger" | "field";


export interface Pressed {
  readonly key: string;
  
  
  readonly isComposing: boolean;
}

const PASS: Answer = { kind: "pass" };


export function answer(pressed: Pressed, origin: Origin, cursor: number, count: number): Answer {
  if (pressed.isComposing) return PASS;
  if (origin === "trigger")
    return pressed.key === "Escape" ? { kind: "close", focus: "opener" } : PASS;
  const last = Math.max(0, count - 1);
  switch (pressed.key) {
    case "ArrowDown":
      return { kind: "cursor", at: Math.min(last, cursor + 1) };
    case "ArrowUp":
      return { kind: "cursor", at: Math.max(0, cursor - 1) };
    case "Home":
      return { kind: "cursor", at: 0 };
    case "End":
      return { kind: "cursor", at: last };
    case "Enter":
      return { kind: "take" };
    case "Escape":
      return { kind: "close", focus: "opener" };
    case "Tab":
      return { kind: "close", focus: "leave" };
    default:
      return PASS;
  }
}


export function claims(answered: Answer): boolean {
  switch (answered.kind) {
    case "cursor":
    case "take":
      return true;
    case "close":
      return answered.focus === "opener";
    case "pass":
      return false;
  }
}


export interface Keyed extends Pressed {
  preventDefault(): void;
}


export type Layer = "unopened" | "open" | "shut";


export interface ComboboxView {
  readonly props: ComboboxProps;
  
  
  readonly uid: string;
  readonly layer: Layer;
  readonly query: string;
  
  readonly cursor: number;
  readonly rows: readonly Choice[];
  readonly side: Side;
}


export interface Hands {
  readonly toggle: () => void;
  readonly act: (answered: Answer) => void;
  readonly type: (query: string) => void;
  readonly point: (at: number) => void;
  readonly take: (choice: Choice) => void;
  
  readonly leave: (next: EventTarget | null) => void;
  readonly holds: Holds;
}

export interface Holds {
  readonly root: Attachment<HTMLDivElement>;
  readonly trigger: Attachment<HTMLButtonElement>;
  readonly layer: Attachment<HTMLDivElement>;
  readonly search: Attachment<HTMLInputElement>;
  readonly list: Attachment<HTMLUListElement>;
}


const HOLD = createAttachmentKey();

export interface RootWire {
  readonly onfocusout: (event: { readonly relatedTarget: EventTarget | null }) => void;
  readonly [hold: symbol]: Attachment<HTMLDivElement>;
}

export interface TriggerWire {
  readonly type: "button";
  readonly "aria-label": string;
  readonly onclick: () => void;
  readonly onkeydown: (event: Keyed) => void;
  readonly [hold: symbol]: Attachment<HTMLButtonElement>;
}

export interface LayerWire {
  
  readonly inert: boolean;
  readonly [hold: symbol]: Attachment<HTMLDivElement>;
}

export interface SearchWire {
  readonly role: "combobox";
  readonly "aria-label": string;
  readonly "aria-expanded": boolean;
  readonly "aria-controls": string;
  readonly "aria-activedescendant": string | undefined;
  readonly placeholder: string;
  readonly value: string;
  readonly oninput: (event: { readonly currentTarget: { readonly value: string } }) => void;
  readonly onkeydown: (event: Keyed) => void;
  readonly [hold: symbol]: Attachment<HTMLInputElement>;
}

export interface ListWire {
  readonly id: string;
  readonly role: "listbox";
  readonly "aria-label": string;
  
  
  readonly onmousedown: (event: { preventDefault(): void }) => void;
  readonly [hold: symbol]: Attachment<HTMLUListElement>;
}

export interface OptionWire {
  readonly id: string;
  readonly role: "option";
  readonly "aria-selected": boolean;
  readonly onmouseenter: () => void;
  readonly onclick: () => void;
}

export interface OptionLook {
  readonly key: string;
  readonly label: string;
  readonly note: string | undefined;
  
  readonly face: string | undefined;
  
  readonly chosen: boolean;
  
  readonly cursor: boolean;
  readonly wire: OptionWire;
}

export interface ComboboxLook {
  readonly root: RootWire;
  readonly trigger: TriggerWire;
  
  readonly face: string;
  
  readonly faceStyle: string | undefined;
  readonly unset: boolean;
  
  readonly layer: LayerWire | undefined;
  readonly open: boolean;
  readonly side: Side;
  readonly search: SearchWire;
  readonly list: ListWire;
  readonly options: readonly OptionLook[];
  readonly empty: string;
}


export const rowId = (uid: string, index: number): string => `${uid}-${String(index)}`;


export function lookOf(view: ComboboxView, hands: Hands): ComboboxLook {
  const { props, uid, layer, cursor, rows } = view;
  const open = layer === "open";
  const chosen = props.choices.find((each) => each.value === props.value);
  const keyed =
    (origin: Origin) =>
    (event: Keyed): void => {
      if (!open) return;
      const answered = answer(event, origin, cursor, rows.length);
      if (claims(answered)) event.preventDefault();
      hands.act(answered);
    };
  return {
    root: {
      onfocusout: (event) => {
        hands.leave(event.relatedTarget);
      },
      [HOLD]: hands.holds.root,
    },
    trigger: {
      type: "button",
      "aria-label": props.label,
      onclick: hands.toggle,
      onkeydown: keyed("trigger"),
      [HOLD]: hands.holds.trigger,
    },
    face: chosen?.label ?? props.placeholder,
    faceStyle: chosen?.face,
    unset: chosen === undefined,
    layer: layer === "unopened" ? undefined : { inert: !open, [HOLD]: hands.holds.layer },
    open,
    side: view.side,
    search: {
      role: "combobox",
      "aria-label": props.label,
      "aria-expanded": open,
      "aria-controls": `${uid}-list`,
      "aria-activedescendant": open && rows.length > 0 ? rowId(uid, cursor) : undefined,
      placeholder: props.placeholder,
      value: view.query,
      oninput: (event) => {
        hands.type(event.currentTarget.value);
      },
      onkeydown: keyed("field"),
      [HOLD]: hands.holds.search,
    },
    list: {
      id: `${uid}-list`,
      role: "listbox",
      "aria-label": props.label,
      onmousedown: (event) => {
        event.preventDefault();
      },
      [HOLD]: hands.holds.list,
    },
    options: rows.map((choice, index) => ({
      key: choice.value,
      label: choice.label,
      note: choice.note,
      face: choice.face,
      chosen: choice.value === props.value,
      cursor: open && index === cursor,
      wire: {
        id: rowId(uid, index),
        role: "option",
        "aria-selected": choice.value === props.value,
        onmouseenter: () => {
          hands.point(index);
        },
        onclick: () => {
          hands.take(choice);
        },
      },
    })),
    empty: props.empty,
  };
}
