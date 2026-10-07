// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The box a key opens over the page - the palette (Accel-K) and the file
// finder (Accel-P) - as one wiring that both seats build and one look
// draws (`modal.look.svelte`).
//
// **The platform owns the modality**, as it does for `parts/dialog`,
// the key sheet and the settings panel: `showModal()` puts the box in
// the top layer, traps the focus, marks the page behind `inert`, and
// `close()` at teardown gives the focus back to whatever held it before
// the box opened (client/Spec.lean §4-64b). Escape arrives as a cancel
// request, which is handed to the caller, because the caller's state
// says whether the box stands and the element closing itself would
// leave that state saying it is open.
//
// **Two seats**, as the key sheet has: `modal` is the one above;
// `specimen` is the same box drawn open in the page's flow for
// `#/gallery`, where a top-layer box would cover every other fold and
// dim it. The specimen takes no focus and answers no Escape.

import { createAttachmentKey } from "svelte/attachments";
import type { Attachment } from "svelte/attachments";

export type Seat = "modal" | "specimen";

// What a click on the dimmed page behind the box does. The palette is a
// place to go, and missing it is a way of saying "nowhere"; the finder
// keeps a half-typed name, so a stray click does not throw it away.
export type Backdrop = "closes" | "stays";

// The name the box is announced by: its own words, or the id of a
// heading inside it.
export type Named = { readonly "aria-label": string } | { readonly "aria-labelledby": string };

// The bag spread on the `<dialog>`. Its symbol key is the attachment
// that opens the element as a modal and closes it at teardown, so the
// look holds no element reference and calls nothing.
export type ModalWire = Named & {
  readonly open: boolean;
  readonly oncancel: (event: Pick<Event, "preventDefault">) => void;
  readonly onclick: (event: Pick<Event, "target" | "currentTarget">) => void;
  readonly [hold: symbol]: Attachment<HTMLDialogElement>;
};

export interface ModalOrder {
  readonly seat: Seat;
  readonly named: Named;
  readonly backdrop: Backdrop;
  readonly onClose: () => void;
  // From `modalHold`, made once for the box's life.
  readonly hold: Attachment<HTMLDialogElement>;
}

const HOLD = createAttachmentKey();

// The attachment that opens the element as a modal and closes it at
// teardown. The seat makes it once and hands the same one to every
// `modalWire`, because Svelte runs an attachment again whenever it is a
// different function, and that would close the box and open it again
// on every redraw.
export function modalHold(seat: Seat): Attachment<HTMLDialogElement> {
  return (node) => {
    if (seat === "specimen") return;
    node.showModal();
    return () => {
      node.close();
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
      // The box fills the element edge to edge, so the only click that
      // lands on the element itself is a click on its backdrop.
      if (backdrop === "closes" && seat === "modal" && event.target === event.currentTarget) onClose();
    },
    [HOLD]: order.hold,
  };
}
