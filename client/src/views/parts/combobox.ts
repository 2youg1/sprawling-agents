// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What a combobox decides before anything is drawn - which rows a
// filter keeps, what each key does - and the whole value its look is
// given (`ComboboxLook`). The look spreads each wire bag on the element
// it names and draws nothing it was not given, so a look built from a
// component library draws the same control by taking the same value.
// The key table is `client/spec/Views/Parts.lean` §7-5, modelled in
// `client/spec/Views/Parts/Combobox.lean`.

import { createAttachmentKey } from "svelte/attachments";
import type { Attachment } from "svelte/attachments";

import type { Side } from "./layer";

export interface Choice {
  readonly value: string;
  // Already in the person's language, or an identifier they typed.
  readonly label: string;
  // The second line: a provider, a price, a context window.
  readonly note?: string;
}

export interface ComboboxProps {
  // The accessible name of the control.
  readonly label: string;
  // What the trigger says while nothing is chosen, and what the search
  // box says while it is empty.
  readonly placeholder: string;
  // What stands in the list when the search matches nothing.
  readonly empty: string;
  readonly choices: readonly Choice[];
  // The value in force; the cursor is never marked as chosen.
  readonly value: string | null;
  // The picked value; closing and focus return stay in the seat.
  readonly onPick: (value: string) => void;
  // How the popup is drawn before anyone touches it. Every screen
  // starts closed; the gallery draws it open so the list is measured.
  readonly starts?: "open" | "closed";
}

// The rows a filter keeps: every choice whose label, note or value
// holds the trimmed filter, without regard to case.
export function matching(choices: readonly Choice[], query: string): readonly Choice[] {
  const needle = query.trim().toLowerCase();
  if (needle === "") return choices;
  return choices.filter((choice) =>
    `${choice.label} ${choice.note ?? ""} ${choice.value}`.toLowerCase().includes(needle),
  );
}

// Where the focus goes when the popup closes: back to the trigger after
// an answer (Enter, Escape), or wherever a leaving focus was already
// going (Tab, a click elsewhere).
export type Return = "opener" | "leave";

// What one key does while the popup is open.
export type Answer =
  | { readonly kind: "cursor"; readonly at: number }
  | { readonly kind: "take" }
  | { readonly kind: "close"; readonly focus: Return }
  | { readonly kind: "pass" };

// Where the key landed: the trigger answers Escape alone, so that the
// popup closes from there too; every other key there is the platform's.
export type Origin = "trigger" | "field";

const PASS: Answer = { kind: "pass" };

// The key table: `cursor` is where the cursor stands now, `count` how
// many rows the filter left. The arrows clamp at both ends.
export function answer(key: string, origin: Origin, cursor: number, count: number): Answer {
  if (origin === "trigger") return key === "Escape" ? { kind: "close", focus: "opener" } : PASS;
  const last = Math.max(0, count - 1);
  switch (key) {
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

// Whether the component takes the key from the platform. Tab closes
// the popup and still moves the focus on, so it is not taken.
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

// A key as the wiring reads it; a `KeyboardEvent` is one.
export interface Keyed {
  readonly key: string;
  preventDefault(): void;
}

// Where the popup is: never opened since the control was mounted, so
// a page of closed comboboxes builds none of their lists; open; or
// closed, keeping the rows it last showed while its look draws it away.
export type Layer = "unopened" | "open" | "shut";

// The state the seat holds, as the wiring reads it.
export interface ComboboxView {
  readonly props: ComboboxProps;
  // The instance's id root: the list and its rows derive their ids
  // from it, so `aria-controls` and `aria-activedescendant` name
  // elements that are there.
  readonly uid: string;
  readonly layer: Layer;
  readonly query: string;
  // Already clamped to the rows.
  readonly cursor: number;
  readonly rows: readonly Choice[];
  readonly side: Side;
}

// What the seat lends the wiring: what to do with an answered key, and
// the elements it has to hold.
export interface Hands {
  readonly toggle: () => void;
  readonly act: (answered: Answer) => void;
  readonly type: (query: string) => void;
  readonly point: (at: number) => void;
  readonly take: (choice: Choice) => void;
  // The focus left an element of the control for `next`.
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

// One key for every attachment a bag carries: a bag carries at most
// one, and a key made once keeps Svelte from letting go of an element
// and taking it again on every draw.
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
  // A leaving popup takes no focus and no pointer, and says nothing.
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
  // A pick by pointer must not move the focus first: the blur would
  // close the popup before the click landed.
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
  // The value in force, which `aria-selected` says too.
  readonly chosen: boolean;
  // Where the cursor is, which `aria-activedescendant` says too.
  readonly cursor: boolean;
  readonly wire: OptionWire;
}

export interface ComboboxLook {
  readonly root: RootWire;
  readonly trigger: TriggerWire;
  // What the trigger says: the chosen label, or the placeholder.
  readonly face: string;
  readonly unset: boolean;
  // Absent until the popup first opens.
  readonly layer: LayerWire | undefined;
  readonly open: boolean;
  readonly side: Side;
  readonly search: SearchWire;
  readonly list: ListWire;
  readonly options: readonly OptionLook[];
  readonly empty: string;
}

// The id of the row at `index`, which the seat finds to scroll it into
// view and the search box names as `aria-activedescendant`.
export const rowId = (uid: string, index: number): string => `${uid}-${String(index)}`;

// The whole value a look draws, wired to `hands`.
export function lookOf(view: ComboboxView, hands: Hands): ComboboxLook {
  const { props, uid, layer, cursor, rows } = view;
  const open = layer === "open";
  const chosen = props.choices.find((each) => each.value === props.value);
  const keyed =
    (origin: Origin) =>
    (event: Keyed): void => {
      if (!open) return;
      const answered = answer(event.key, origin, cursor, rows.length);
      if (claims(answered)) event.preventDefault();
      hands.act(answered);
    };
  return {
    root: { onfocusout: (event) => { hands.leave(event.relatedTarget); }, [HOLD]: hands.holds.root },
    trigger: {
      type: "button",
      "aria-label": props.label,
      onclick: hands.toggle,
      onkeydown: keyed("trigger"),
      [HOLD]: hands.holds.trigger,
    },
    face: chosen?.label ?? props.placeholder,
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
      oninput: (event) => { hands.type(event.currentTarget.value); },
      onkeydown: keyed("field"),
      [HOLD]: hands.holds.search,
    },
    list: {
      id: `${uid}-list`,
      role: "listbox",
      "aria-label": props.label,
      onmousedown: (event) => { event.preventDefault(); },
      [HOLD]: hands.holds.list,
    },
    options: rows.map((choice, index) => ({
      key: choice.value,
      label: choice.label,
      note: choice.note,
      chosen: choice.value === props.value,
      cursor: open && index === cursor,
      wire: {
        id: rowId(uid, index),
        role: "option",
        "aria-selected": choice.value === props.value,
        onmouseenter: () => { hands.point(index); },
        onclick: () => { hands.take(choice); },
      },
    })),
    empty: props.empty,
  };
}
