// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { ElementRef } from "../core/element_ref";

export type Seat = "modal" | "specimen";

export type First = "control" | "sheet";

export type Backdrop = "closes" | "stays";

export type Named = { readonly "aria-label": string } | { readonly "aria-labelledby": string };

export type ModalWire = Named & {
  readonly open: boolean;
  readonly oncancel: (event: Pick<Event, "preventDefault">) => void;
  readonly onclick: (event: Pick<Event, "target" | "currentTarget">) => void;
  readonly [hold: symbol]: ElementRef<HTMLDialogElement>;
};

export interface ModalOrder {
  readonly seat: Seat;
  readonly named: Named;
  readonly backdrop: Backdrop;
  readonly onClose: () => void;

  readonly hold: ElementRef<HTMLDialogElement>;
}

const HOLD = Symbol();

export function modalHold(seat: Seat, first: First = "control"): ElementRef<HTMLDialogElement> {
  return (node) => {
    if (seat === "specimen") return;

    if (first === "sheet") {
      node.autofocus = true;
      node.tabIndex = -1;
    }
    const before = document.activeElement;
    node.showModal();
    if (first === "sheet") node.focus();
    return () => {
      node.close();

      const lost = document.activeElement === null || document.activeElement === document.body;
      if (lost && before instanceof HTMLElement && before.isConnected) before.focus();
    };
  };
}

export function modalWire(order: ModalOrder): ModalWire {
  const { seat, named, backdrop, onClose } = order;
  return {
    ...named,
    open: seat === "specimen",
    oncancel: (event) => {
      event.preventDefault();
      if (seat === "modal") onClose();
    },
    onclick: (event) => {
      if (backdrop === "closes" && seat === "modal" && event.target === event.currentTarget)
        onClose();
    },
    [HOLD]: order.hold,
  };
}
