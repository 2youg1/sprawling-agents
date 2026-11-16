// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createAttachmentKey } from "svelte/attachments";
import type { Attachment } from "svelte/attachments";

import type { Cancelling, SheetWire, Stands } from "./sheet";

export type Seat = "modal" | "specimen";

export interface KbdLook {
  readonly marks: readonly string[];
  readonly class?: string | undefined;
}

export interface Said {
  readonly id: string;
  readonly text: string;
}

export interface Row {
  readonly key: string;
  readonly label: string;
  readonly marks: readonly string[];
}

export interface CheatsheetLook {
  readonly sheet: SheetWire;
  readonly stands: Stands;
  readonly heading: Said;
  readonly dismiss: { readonly label: string; readonly onPress: () => void };
  readonly rows: readonly Row[];
  readonly where: Said;
}

export interface Sheeting {
  readonly uid: string;
  readonly seat: Seat;
  readonly onClose: () => void;
  readonly hold: Attachment<HTMLDialogElement>;
}

export interface Words {
  readonly title: string;
  readonly dismiss: string;
  readonly where: string;
  readonly rows: readonly Row[];
}

const STANDS: Readonly<Record<Seat, Stands>> = {
  modal: "below-top",
  specimen: "in-flow",
};

const HOLD = createAttachmentKey();

export function sheetOf(sheeting: Sheeting, words: Words): CheatsheetLook {
  const heading = { id: `${sheeting.uid}-title`, text: words.title };
  const where = { id: `${sheeting.uid}-where`, text: words.where };
  return {
    sheet: {
      [HOLD]: sheeting.hold,
      open: sheeting.seat === "specimen",
      "aria-labelledby": heading.id,
      "aria-describedby": where.id,
      oncancel: (event: Cancelling) => {
        event.preventDefault();
        sheeting.onClose();
      },
    },
    stands: STANDS[sheeting.seat],
    heading,
    dismiss: { label: words.dismiss, onPress: sheeting.onClose },
    rows: words.rows,
    where,
  };
}
