// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The settings panel's `<dialog>` (client/Spec.lean §7L): what its look
// is given, and the bag of wiring the look spreads on the element.

import type { Snippet } from "svelte";
import { createAttachmentKey, type Attachment } from "svelte/attachments";

// What a cancel or a click hands the handler; nothing more, so the
// wiring test needs no DOM.
export interface Cancel {
  readonly preventDefault: () => void;
}
export interface Click {
  readonly target: unknown;
  readonly currentTarget: unknown;
}

// The bag spread on the `<dialog>`. Its symbol key is a Svelte
// attachment that hands the seat the element it opens as a modal.
export interface DialogWire {
  readonly "aria-labelledby": string;
  readonly oncancel: (event: Cancel) => void;
  readonly onclick: (event: Click) => void;
  readonly [hold: symbol]: Attachment<HTMLDialogElement>;
}

export interface PanelLook {
  readonly dialog: DialogWire;
  readonly children: Snippet;
}

const HOLD = createAttachmentKey();

// The dialog's wiring. Escape asks the caller, which moves the address
// bar, rather than closing the element: a closed element under an
// address that says open would disagree with the address. A click whose
// target is the dialog itself landed on the backdrop, because the inner
// frame fills the whole element, and that is the one click that closes.
export function dialogOf(titleId: string, onClose: () => void, hold: Attachment<HTMLDialogElement>): DialogWire {
  return {
    "aria-labelledby": titleId,
    oncancel: (event) => {
      event.preventDefault();
      onClose();
    },
    onclick: (event) => {
      if (event.target === event.currentTarget) onClose();
    },
    [HOLD]: hold,
  };
}
