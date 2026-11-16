// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Snippet } from "svelte";
import { createAttachmentKey, type Attachment } from "svelte/attachments";

export interface Cancel {
  readonly preventDefault: () => void;
}
export interface Click {
  readonly target: unknown;
  readonly currentTarget: unknown;
}

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

export function dialogOf(
  titleId: string,
  onClose: () => void,
  hold: Attachment<HTMLDialogElement>,
): DialogWire {
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
