// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the two looks of `kbd.svelte` are handed: the marks of one chord
// (`kbd.look.svelte`), and the sheet every chord is read on
// (`kbd_sheet.look.svelte`). The marks are read from `core/keys` by the
// seat, so a look never spells a key of its own (client/Spec.lean §7-1).

import { createAttachmentKey } from "svelte/attachments";
import type { Attachment } from "svelte/attachments";

import type { Cancelling, SheetWire, Stands } from "./sheet";

// Where the sheet stands: in the top layer as the page's one modal, or
// in the gallery's flow as a picture of itself.
export type Seat = "modal" | "specimen";

export interface KbdLook {
  // One glyph per `<kbd>`: `⌘` and `K`, or `Ctrl` and `K`.
  readonly marks: readonly string[];
  readonly class?: string | undefined;
}

export interface Said {
  readonly id: string;
  readonly text: string;
}

export interface Row {
  // The action's own name, unique in the table.
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

// What the seat hands `sheetOf` beside the words: its id, where it
// stands, the way it is closed and how it holds its element.
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

// The sheet's wiring. A specimen is `<dialog open>` in the page's flow
// and takes no focus; the modal one is opened by the seat with
// `showModal()`. Escape reaches either as a cancel request, which is
// answered by the caller rather than by the element, and a click on the
// backdrop closes nothing - a sheet of key chords is not a question one
// answers by missing it (client/Spec.lean §7-3).
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
