// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The question asked before something that cannot be taken back, as its
// look is handed it (client/Spec.lean §7-3): the relations on the
// `<dialog>`, the cancel request answered by the caller, and the two
// answers in the order the platform's focus reads them.

import { createAttachmentKey } from "svelte/attachments";
import type { Attachment } from "svelte/attachments";

import type { Cancelling, SheetWire, Stands } from "./sheet";

export interface DialogProps {
  readonly open: boolean;
  // Already in the person's language: what is about to happen.
  readonly title: string;
  // What it will cost, in the words of the page.
  readonly detail?: string;
  readonly confirmLabel: string;
  readonly cancelLabel: string;
  readonly onConfirm: () => void;
  readonly onCancel: () => void;
  // Whether the confirming answer destroys something.
  readonly destructive?: boolean;
}

export interface Said {
  readonly id: string;
  readonly text: string;
}

export interface Answer {
  readonly label: string;
  readonly tone: "secondary" | "primary" | "destructive";
  readonly onPress: () => void;
}

export interface DialogLook {
  readonly sheet: SheetWire;
  readonly stands: Stands;
  readonly heading: Said;
  readonly detail: Said | undefined;
  // The way out first: the platform focuses the first control inside
  // the dialog, so document order is what puts the safe answer under
  // the hand on a question about deleting something.
  readonly answers: readonly [Answer, Answer];
}

// One key for every instance, so a rebuilt look does not tear the
// element's hold down and put it back.
const HOLD = createAttachmentKey();

export function lookOf(props: DialogProps, uid: string, hold: Attachment<HTMLDialogElement>): DialogLook {
  const heading = { id: `${uid}-title`, text: props.title };
  const detail = props.detail === undefined ? undefined : { id: `${uid}-detail`, text: props.detail };
  return {
    sheet: {
      [HOLD]: hold,
      "aria-labelledby": heading.id,
      "aria-describedby": detail?.id,
      // Escape reaches here as a cancel request. The default would close
      // the element behind the caller's back and leave `open` saying it
      // is still up, so the request is answered by the caller instead.
      oncancel: (event: Cancelling) => {
        event.preventDefault();
        props.onCancel();
      },
    },
    stands: "centre",
    heading,
    detail,
    answers: [
      { label: props.cancelLabel, tone: "secondary", onPress: props.onCancel },
      { label: props.confirmLabel, tone: props.destructive === true ? "destructive" : "primary", onPress: props.onConfirm },
    ],
  };
}
